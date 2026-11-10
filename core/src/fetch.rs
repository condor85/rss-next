use crate::discover::{common_feed_urls, discover_feeds};
use crate::error::{net, Error};
use crate::model::{FeedData, FeedOutcome};
use crate::parse::parse_feed;
use futures::{stream, StreamExt};
use reqwest::{header, redirect::Policy, Client};
use std::time::Duration;
use url::Url;

pub const USER_AGENT: &str = "Mozilla/5.0 (Linux; Android 14; Pixel) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Mobile Safari/537.36 LettoreRSS/0.1";
const MAX_BYTES: usize = 8 * 1024 * 1024;
const MAX_DISCOVERY_TRIES: usize = 5;

/// Client HTTP con timeout, redirect, decompressione gzip/brotli e certificati integrati
/// (rustls + webpki: non dipende dal sistema, quindi funziona uguale su Android).
pub fn build_client(timeout: Duration) -> Result<Client, Error> {
    let mut h = header::HeaderMap::new();
    h.insert(
        header::ACCEPT,
        header::HeaderValue::from_static(
            "application/rss+xml, application/atom+xml, application/feed+json, application/xml;q=0.9, text/xml;q=0.8, text/html;q=0.7, */*;q=0.5",
        ),
    );
    Client::builder()
        .user_agent(USER_AGENT)
        .default_headers(h)
        .timeout(timeout)
        .connect_timeout(Duration::from_secs(10))
        .redirect(Policy::limited(8))
        .build()
        .map_err(net)
}

fn check_url(u: &str) -> Result<Url, Error> {
    let p = Url::parse(u.trim()).map_err(|_| Error::BadUrl(u.to_string()))?;
    if matches!(p.scheme(), "http" | "https") {
        Ok(p)
    } else {
        Err(Error::BadUrl(u.to_string()))
    }
}

/// Aggiunge https:// se manca lo schema ("example.com/feed" → "https://example.com/feed").
pub fn normalize_url(input: &str) -> String {
    let t = input.trim();
    if t.starts_with("http://") || t.starts_with("https://") {
        t.to_string()
    } else {
        format!("https://{t}")
    }
}

async fn get_once(client: &Client, url: &Url) -> Result<Vec<u8>, Error> {
    let mut resp = client.get(url.clone()).send().await.map_err(net)?;
    let status = resp.status();
    if !status.is_success() {
        return Err(Error::Status(status.as_u16()));
    }
    if resp
        .content_length()
        .is_some_and(|n| n as usize > MAX_BYTES)
    {
        return Err(Error::TooLarge);
    }
    let mut buf = Vec::new();
    while let Some(chunk) = resp.chunk().await.map_err(net)? {
        if buf.len() + chunk.len() > MAX_BYTES {
            return Err(Error::TooLarge);
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(buf)
}

/// Scarica un URL; un solo nuovo tentativo per errori temporanei (rete, 5xx).
pub async fn get_bytes(client: &Client, url: &str) -> Result<Vec<u8>, Error> {
    let parsed = check_url(url)?;
    match get_once(client, &parsed).await {
        Err(e) if e.is_transient() => {
            tokio::time::sleep(Duration::from_millis(600)).await;
            get_once(client, &parsed).await
        }
        other => other,
    }
}

fn looks_like_html(bytes: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]).to_ascii_lowercase();
    head.contains("<html") || head.contains("<!doctype html") || head.contains("<head")
}

/// Scarica e interpreta un feed. Se l'URL è una pagina web, cerca il feed dichiarato nella pagina
/// (o in qualche percorso comune) e usa quello: `FeedData::url` contiene l'URL del feed vero.
pub async fn fetch_feed(client: &Client, url: &str) -> Result<FeedData, Error> {
    let bytes = get_bytes(client, url).await?;
    match parse_feed(url, &bytes) {
        Ok(f) => Ok(f),
        Err(Error::NotAFeed) if looks_like_html(&bytes) => {
            let html = String::from_utf8_lossy(&bytes);
            let mut candidates: Vec<String> = discover_feeds(&html, url)
                .into_iter()
                .map(|c| c.url)
                .collect();
            if candidates.is_empty() {
                candidates = common_feed_urls(url);
            }
            for cand in candidates.into_iter().take(MAX_DISCOVERY_TRIES) {
                if cand == url {
                    continue;
                }
                if let Ok(b) = get_bytes(client, &cand).await {
                    if let Ok(f) = parse_feed(&cand, &b) {
                        return Ok(f);
                    }
                }
            }
            Err(Error::NotAFeed)
        }
        Err(e) => Err(e),
    }
}

/// Aggiorna molti feed in parallelo (al massimo `concurrency` insieme).
/// Ogni feed ha il suo esito: un errore su uno non tocca gli altri.
pub async fn refresh_all(
    client: &Client,
    urls: Vec<String>,
    concurrency: usize,
) -> Vec<FeedOutcome> {
    let mut unique: Vec<String> = Vec::with_capacity(urls.len());
    for u in urls {
        if !unique.contains(&u) {
            unique.push(u);
        }
    }
    stream::iter(unique)
        .map(|u| {
            let c = client.clone();
            async move {
                match fetch_feed(&c, &u).await {
                    Ok(f) => FeedOutcome {
                        url: u,
                        ok: true,
                        error: None,
                        feed: Some(f),
                    },
                    Err(e) => FeedOutcome {
                        url: u,
                        ok: false,
                        error: Some(e.to_string()),
                        feed: None,
                    },
                }
            }
        })
        .buffer_unordered(concurrency.max(1))
        .collect()
        .await
}

/// Aggiunge un feed a partire da quello che ha scritto l'utente: normalizza l'indirizzo e lo scarica
/// (scoprendo il feed se è una pagina web). `FeedOutcome::url` è l'indirizzo normalizzato richiesto,
/// `feed.url` quello del feed vero.
pub async fn add_feed(client: &Client, input: &str) -> FeedOutcome {
    if input.trim().is_empty() {
        return FeedOutcome {
            url: String::new(),
            ok: false,
            error: Some(Error::Empty.to_string()),
            feed: None,
        };
    }
    let url = normalize_url(input);
    match fetch_feed(client, &url).await {
        Ok(f) => FeedOutcome {
            url,
            ok: true,
            error: None,
            feed: Some(f),
        },
        Err(e) => FeedOutcome {
            url,
            ok: false,
            error: Some(e.to_string()),
            feed: None,
        },
    }
}
