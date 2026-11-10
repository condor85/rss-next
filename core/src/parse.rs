use crate::error::Error;
use crate::media::bare_thumbnails;
use crate::model::{Article, FeedData};
use crate::text::{first_image, html_to_text, truncate_chars};
use feed_rs::{model as fm, parser};
use std::collections::HashMap;

const MAX_ARTICLES: usize = 100;
const MAX_SUMMARY: usize = 600;

fn is_image_url(u: &str) -> bool {
    let path = u.split(['?', '#']).next().unwrap_or(u).to_ascii_lowercase();
    [".jpg", ".jpeg", ".png", ".webp", ".gif", ".avif"]
        .iter()
        .any(|e| path.ends_with(e))
}

fn pick_link(links: &[fm::Link]) -> Option<&fm::Link> {
    links
        .iter()
        .find(|l| matches!(l.rel.as_deref(), None | Some("alternate")))
        .or_else(|| links.first())
}

fn entry_image(e: &fm::Entry, html: &str) -> Option<String> {
    for m in &e.media {
        if let Some(t) = m.thumbnails.first() {
            return Some(t.image.uri.clone());
        }
        for c in &m.content {
            if let Some(u) = &c.url {
                let u = u.to_string();
                let is_img = c
                    .content_type
                    .as_ref()
                    .map(|t| t.to_string().starts_with("image/"))
                    .unwrap_or(false);
                if is_img || is_image_url(&u) {
                    return Some(u);
                }
            }
        }
    }
    first_image(html)
}

fn convert_entry(e: &fm::Entry, thumbs: &HashMap<String, String>) -> Article {
    let link = pick_link(&e.links)
        .map(|l| l.href.clone())
        .unwrap_or_default();
    let title = e
        .title
        .as_ref()
        .map(|t| html_to_text(&t.content))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "(senza titolo)".to_string());

    let summary_html = e.summary.as_ref().map(|s| s.content.as_str()).unwrap_or("");
    let content_html = e
        .content
        .as_ref()
        .and_then(|c| c.body.as_deref())
        .unwrap_or("");
    let mut summary = html_to_text(summary_html);
    if summary.is_empty() {
        summary = html_to_text(content_html);
    }
    if summary.is_empty() {
        summary = e
            .media
            .iter()
            .find_map(|m| m.description.as_ref())
            .map(|d| html_to_text(&d.content))
            .unwrap_or_default();
    }
    let image_src = if content_html.is_empty() {
        summary_html
    } else {
        content_html
    };
    let image = entry_image(e, image_src)
        .or_else(|| thumbs.get(&e.id).cloned())
        .or_else(|| thumbs.get(&link).cloned())
        .or_else(|| first_image(summary_html));

    let id = if !e.id.is_empty() {
        e.id.clone()
    } else if !link.is_empty() {
        link.clone()
    } else {
        title.clone()
    };

    Article {
        id,
        title,
        link,
        date: e
            .published
            .or(e.updated)
            .map(|d| d.timestamp_millis())
            .unwrap_or(0),
        summary: truncate_chars(&summary, MAX_SUMMARY),
        image,
    }
}

/// Interpreta il contenuto (RSS 0.9/1.0/2.0, Atom, JSON Feed) scaricato da `url`.
pub fn parse_feed(url: &str, bytes: &[u8]) -> Result<FeedData, Error> {
    let feed = parser::Builder::new()
        .base_uri(Some(url))
        .build()
        .parse(bytes)
        .map_err(|_| Error::NotAFeed)?;

    let host = url::Url::parse(url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string));
    let title = feed
        .title
        .as_ref()
        .map(|t| html_to_text(&t.content))
        .filter(|s| !s.is_empty())
        .or(host)
        .unwrap_or_else(|| url.to_string());
    let site = pick_link(&feed.links)
        .map(|l| l.href.clone())
        .filter(|h| h.starts_with("http"));

    let thumbs = bare_thumbnails(bytes);
    let mut articles: Vec<Article> = feed
        .entries
        .iter()
        .map(|e| convert_entry(e, &thumbs))
        .collect();
    // Più recenti prima; gli articoli senza data in fondo (sort stabile: l'ordine del feed resta).
    articles.sort_by_key(|a| std::cmp::Reverse(a.date));
    articles.truncate(MAX_ARTICLES);

    Ok(FeedData {
        url: url.to_string(),
        title,
        site,
        articles,
    })
}

/// Interpreta un feed salvato in un file (senza indirizzo): l'URL "virtuale" è `file:<nome>`.
pub fn parse_local(name: &str, bytes: &[u8]) -> Result<FeedData, Error> {
    parse_feed(&format!("file:{name}"), bytes)
}
