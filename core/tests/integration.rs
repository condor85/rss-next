//! Test con un vero server HTTP locale: niente rete esterna, ma tutto il percorso reale
//! (reqwest → byte → parsing → esito) viene esercitato.

use rss_core::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[derive(Clone)]
struct Route {
    status: u16,
    ctype: &'static str,
    body: Vec<u8>,
    delay_ms: u64,
    track: bool,
}

fn route(ctype: &'static str, body: impl Into<Vec<u8>>) -> Route {
    Route {
        status: 200,
        ctype,
        body: body.into(),
        delay_ms: 0,
        track: false,
    }
}

async fn bind() -> (TcpListener, String) {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://127.0.0.1:{}", l.local_addr().unwrap().port());
    (l, base)
}

/// Avvia il server; restituisce il contatore del massimo di richieste "tracciate" contemporanee.
fn serve(listener: TcpListener, routes: HashMap<String, Route>) -> Arc<AtomicUsize> {
    let routes = Arc::new(routes);
    let cur = Arc::new(AtomicUsize::new(0));
    let max = Arc::new(AtomicUsize::new(0));
    let max_out = max.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                break;
            };
            let (routes, cur, max) = (routes.clone(), cur.clone(), max.clone());
            tokio::spawn(async move {
                let mut buf = vec![0u8; 8192];
                let mut n = 0;
                loop {
                    let r = sock.read(&mut buf[n..]).await.unwrap_or(0);
                    if r == 0 {
                        break;
                    }
                    n += r;
                    if buf[..n].windows(4).any(|w| w == b"\r\n\r\n") || n == buf.len() {
                        break;
                    }
                }
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let path = req.split_whitespace().nth(1).unwrap_or("/").to_string();
                let r = routes.get(&path).cloned().unwrap_or(Route {
                    status: 404,
                    ctype: "text/plain",
                    body: b"not found".to_vec(),
                    delay_ms: 0,
                    track: false,
                });
                if r.track {
                    let c = cur.fetch_add(1, SeqCst) + 1;
                    max.fetch_max(c, SeqCst);
                }
                if r.delay_ms > 0 {
                    tokio::time::sleep(Duration::from_millis(r.delay_ms)).await;
                }
                if r.track {
                    cur.fetch_sub(1, SeqCst);
                }
                let head = format!(
                    "HTTP/1.1 {} X\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    r.status,
                    r.ctype,
                    r.body.len()
                );
                let _ = sock.write_all(head.as_bytes()).await;
                let _ = sock.write_all(&r.body).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    max_out
}

fn rss_utf8() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:media="http://search.yahoo.com/mrss/" xmlns:content="http://purl.org/rss/1.0/modules/content/">
<channel>
  <title>Notizie d'Italia</title>
  <link>https://notizie.example/</link>
  <item>
    <title>Perché l'Università è già aperta &amp; attiva</title>
    <link>https://notizie.example/a1</link>
    <guid isPermaLink="false">a1</guid>
    <pubDate>Tue, 03 Nov 2026 09:00:00 +0000</pubDate>
    <description><![CDATA[<p>Primo <b>paragrafo</b>.</p><script>alert(1)</script><p>Secondo&nbsp;paragrafo.</p><img src="https://notizie.example/i/1.jpg">]]></description>
  </item>
  <item>
    <title>Articolo più vecchio</title>
    <link>https://notizie.example/a2</link>
    <guid>a2</guid>
    <pubDate>Mon, 02 Nov 2026 09:00:00 +0000</pubDate>
    <description>Solo testo</description>
    <media:thumbnail url="https://notizie.example/i/2.png"/>
  </item>
  <item>
    <title>Senza data</title>
    <link>https://notizie.example/a3</link>
    <guid>a3</guid>
  </item>
</channel></rss>"#
        .to_string()
}

fn rss_latin1() -> Vec<u8> {
    let s = r#"<?xml version="1.0" encoding="ISO-8859-1"?>
<rss version="2.0"><channel><title>Perché è così</title><link>https://latin.example/</link>
<item><title>Città e perché</title><link>https://latin.example/1</link><guid>l1</guid>
<pubDate>Tue, 03 Nov 2026 09:00:00 +0000</pubDate><description>caffè</description></item>
</channel></rss>"#;
    s.chars().map(|c| c as u32 as u8).collect()
}

fn atom(base: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Blog Atom</title>
  <link rel="self" href="{base}/atom.xml"/>
  <link rel="alternate" href="{base}/"/>
  <updated>2026-11-03T10:00:00Z</updated>
  <id>urn:uuid:blog</id>
  <entry>
    <title>Post uno</title>
    <link href="/post/1"/>
    <id>urn:post:1</id>
    <updated>2026-11-03T10:00:00Z</updated>
    <content type="html">&lt;p&gt;Contenuto &lt;i&gt;html&lt;/i&gt;&lt;/p&gt;</content>
  </entry>
</feed>"#
    )
}

#[tokio::test]
async fn parses_rss_cleans_html_and_orders_by_date() {
    let f = parse_feed("https://notizie.example/feed", rss_utf8().as_bytes()).unwrap();
    assert_eq!(f.title, "Notizie d'Italia");
    assert_eq!(f.site.as_deref(), Some("https://notizie.example/"));
    assert_eq!(f.articles.len(), 3);
    // Ordine: più recente, poi più vecchio, infine senza data.
    let titles: Vec<_> = f.articles.iter().map(|a| a.title.as_str()).collect();
    assert_eq!(
        titles,
        [
            "Perché l'Università è già aperta & attiva",
            "Articolo più vecchio",
            "Senza data"
        ]
    );
    let a = &f.articles[0];
    assert_eq!(a.summary, "Primo paragrafo. Secondo paragrafo.");
    assert!(!a.summary.contains("alert"));
    assert_eq!(a.image.as_deref(), Some("https://notizie.example/i/1.jpg"));
    assert_eq!(a.link, "https://notizie.example/a1");
    assert_eq!(a.date, 1_793_696_400_000); // 2026-11-03T09:00:00Z
    assert_eq!(
        f.articles[1].image.as_deref(),
        Some("https://notizie.example/i/2.png")
    );
    assert_eq!(f.articles[2].date, 0);
}

#[tokio::test]
async fn decodes_latin1_feeds() {
    let f = parse_feed("https://latin.example/feed", &rss_latin1()).unwrap();
    assert_eq!(f.title, "Perché è così");
    assert_eq!(f.articles[0].title, "Città e perché");
    assert_eq!(f.articles[0].summary, "caffè");
}

#[tokio::test]
async fn local_file_feed_parses_with_file_scheme() {
    let f = parse_feed("file:backup.xml", rss_utf8().as_bytes()).unwrap();
    assert_eq!(f.url, "file:backup.xml");
    assert_eq!(f.articles.len(), 3);
}

#[tokio::test]
async fn garbage_is_not_a_feed() {
    assert_eq!(
        parse_feed("https://x.example/", b"<rss><channel>oops").unwrap_err(),
        Error::NotAFeed
    );
    assert_eq!(
        parse_feed("https://x.example/", b"").unwrap_err(),
        Error::NotAFeed
    );
}

#[tokio::test]
async fn atom_relative_links_become_absolute() {
    let (l, base) = bind().await;
    let mut r = HashMap::new();
    r.insert(
        "/atom.xml".to_string(),
        route("application/atom+xml", atom(&base)),
    );
    serve(l, r);
    let client = build_client(Duration::from_secs(5)).unwrap();
    let f = fetch_feed(&client, &format!("{base}/atom.xml"))
        .await
        .unwrap();
    assert_eq!(f.title, "Blog Atom");
    assert_eq!(f.articles[0].link, format!("{base}/post/1"));
    assert_eq!(f.articles[0].summary, "Contenuto html");
}

/// Il bug segnalato dall'utente: "carica un solo feed". Un feed rotto, mancante, lento o
/// enorme non deve MAI impedire il caricamento degli altri.
#[tokio::test]
async fn one_broken_feed_never_blocks_the_others() {
    let (l, base) = bind().await;
    let mut r = HashMap::new();
    r.insert("/ok1.xml".into(), route("application/rss+xml", rss_utf8()));
    r.insert(
        "/ok2.xml".into(),
        route("application/atom+xml", atom(&base)),
    );
    r.insert(
        "/latin.xml".into(),
        route("application/rss+xml; charset=ISO-8859-1", rss_latin1()),
    );
    r.insert(
        "/bad.xml".into(),
        route("application/rss+xml", "<rss><channel>oops"),
    );
    r.insert(
        "/html.html".into(),
        route(
            "text/html",
            "<html><head><title>x</title></head><body>niente</body></html>",
        ),
    );
    r.insert(
        "/big.xml".into(),
        route("application/rss+xml", vec![b'a'; 9 * 1024 * 1024]),
    );
    r.insert(
        "/slow.xml".into(),
        Route {
            delay_ms: 4000,
            ..route("application/rss+xml", rss_utf8())
        },
    );
    serve(l, r);

    let client = build_client(Duration::from_millis(800)).unwrap();
    let urls: Vec<String> = [
        "ok1", "ok2", "latin", "bad", "html", "big", "slow", "missing",
    ]
    .iter()
    .map(|n| format!("{base}/{n}.{}", if *n == "html" { "html" } else { "xml" }))
    .collect();
    let out = refresh_all(&client, urls.clone(), 4).await;

    assert_eq!(out.len(), urls.len(), "un esito per ogni feed richiesto");
    let by: HashMap<_, _> = out.iter().map(|o| (o.url.clone(), o)).collect();
    let get = |n: &str, ext: &str| by[&format!("{base}/{n}.{ext}")];

    for n in ["ok1", "ok2", "latin"] {
        let o = get(n, "xml");
        assert!(o.ok, "{n} doveva riuscire: {:?}", o.error);
        assert!(!o.feed.as_ref().unwrap().articles.is_empty());
    }
    assert_eq!(
        get("bad", "xml").error.as_deref(),
        Some("non è un feed RSS/Atom valido")
    );
    assert_eq!(
        get("html", "html").error.as_deref(),
        Some("non è un feed RSS/Atom valido")
    );
    assert_eq!(
        get("big", "xml").error.as_deref(),
        Some("risposta troppo grande")
    );
    assert_eq!(
        get("slow", "xml").error.as_deref(),
        Some("rete: tempo scaduto")
    );
    assert_eq!(
        get("missing", "xml").error.as_deref(),
        Some("il server ha risposto 404")
    );
    assert_eq!(out.iter().filter(|o| o.ok).count(), 3);
}

#[tokio::test]
async fn concurrency_is_bounded() {
    let (l, base) = bind().await;
    let mut r = HashMap::new();
    for i in 0..12 {
        r.insert(
            format!("/f{i}.xml"),
            Route {
                delay_ms: 120,
                track: true,
                ..route("application/rss+xml", rss_utf8())
            },
        );
    }
    let max = serve(l, r);
    let client = build_client(Duration::from_secs(10)).unwrap();
    let urls: Vec<String> = (0..12).map(|i| format!("{base}/f{i}.xml")).collect();
    let out = refresh_all(&client, urls, 3).await;
    assert_eq!(out.iter().filter(|o| o.ok).count(), 12);
    let peak = max.load(SeqCst);
    assert!(peak <= 3, "picco di richieste contemporanee: {peak}");
    assert!(peak >= 2, "doveva usare il parallelismo, picco {peak}");
}

#[tokio::test]
async fn discovers_the_feed_from_a_web_page() {
    let (l, base) = bind().await;
    let mut r = HashMap::new();
    r.insert(
        "/".into(),
        route(
            "text/html",
            r#"<html><head>
              <link rel="stylesheet" href="/s.css">
              <link rel="alternate" type="application/rss+xml" title="Feed" href="/rss.xml">
            </head><body>ciao</body></html>"#,
        ),
    );
    r.insert("/rss.xml".into(), route("application/rss+xml", rss_utf8()));
    serve(l, r);
    let client = build_client(Duration::from_secs(5)).unwrap();
    let f = fetch_feed(&client, &format!("{base}/")).await.unwrap();
    assert_eq!(
        f.url,
        format!("{base}/rss.xml"),
        "deve restituire l'URL del feed vero"
    );
    assert_eq!(f.articles.len(), 3);
}

#[tokio::test]
async fn falls_back_to_common_feed_paths() {
    let (l, base) = bind().await;
    let mut r = HashMap::new();
    r.insert(
        "/".into(),
        route(
            "text/html",
            "<html><head></head><body>nessun link</body></html>",
        ),
    );
    r.insert("/feed".into(), route("application/rss+xml", rss_utf8()));
    serve(l, r);
    let client = build_client(Duration::from_secs(5)).unwrap();
    let f = fetch_feed(&client, &format!("{base}/")).await.unwrap();
    assert_eq!(f.url, format!("{base}/feed"));
}

#[tokio::test]
async fn rejects_non_http_urls() {
    let client = build_client(Duration::from_secs(2)).unwrap();
    for u in [
        "ftp://x.example/feed",
        "file:///etc/passwd",
        "javascript:alert(1)",
        "non un url",
    ] {
        let e = fetch_feed(&client, u).await.unwrap_err();
        assert!(matches!(e, Error::BadUrl(_)), "{u} → {e:?}");
    }
}

#[test]
fn normalizes_urls() {
    assert_eq!(
        normalize_url("example.com/feed"),
        "https://example.com/feed"
    );
    assert_eq!(normalize_url("  http://a.b/x "), "http://a.b/x");
    assert_eq!(normalize_url("https://a.b"), "https://a.b");
}

#[test]
fn opml_keeps_nested_folders() {
    let xml = r#"<?xml version="1.0"?>
<opml version="2.0"><head><title>x</title></head><body>
  <outline text="Notizie">
    <outline text="ANSA" title="ANSA.it" type="rss" xmlUrl="https://ansa.example/rss.xml"/>
    <outline text="Estero"><outline text="BBC" type="rss" xmlUrl="https://bbc.example/rss"/></outline>
  </outline>
  <outline text="Senza cartella" type="rss" xmlUrl="https://solo.example/feed"/>
</body></opml>"#;
    let f = parse_opml(xml).unwrap();
    assert_eq!(f.len(), 3);
    assert_eq!(
        (f[0].folder.as_str(), f[0].title.as_deref()),
        ("Notizie", Some("ANSA.it"))
    );
    assert_eq!(f[1].folder, "Estero");
    assert_eq!(
        (f[2].folder.as_str(), f[2].url.as_str()),
        (DEFAULT_FOLDER, "https://solo.example/feed")
    );
    assert!(matches!(parse_opml("<opml><body>"), Err(Error::Opml(_))));
}

#[test]
fn opml_export_roundtrip() {
    let feeds = vec![
        ImportedFeed {
            url: "https://a.example/rss".into(),
            title: Some("A & B".into()),
            folder: "Tech".into(),
        },
        ImportedFeed {
            url: "https://b.example/rss".into(),
            title: None,
            folder: "Tech".into(),
        },
        ImportedFeed {
            url: "https://c.example/rss".into(),
            title: Some("C".into()),
            folder: "Sport".into(),
        },
    ];
    let xml = export_opml(&feeds).unwrap();
    let back = parse_opml(&xml).unwrap();
    assert_eq!(back.len(), 3);
    assert_eq!(back[0].title.as_deref(), Some("A & B"));
    assert_eq!(back[2].folder, "Sport");
    assert_eq!(back.iter().filter(|f| f.folder == "Tech").count(), 2);
}

#[test]
fn import_text_handles_url_lists_and_opml() {
    let list = "https://a.example/rss\n  https://b.example/feed.xml, https://a.example/rss\nnota inutile\n(https://c.example/atom).";
    let f = import_text(list, "Miei").unwrap();
    let urls: Vec<_> = f.iter().map(|x| x.url.as_str()).collect();
    assert_eq!(
        urls,
        [
            "https://a.example/rss",
            "https://b.example/feed.xml",
            "https://c.example/atom"
        ]
    );
    assert!(f.iter().all(|x| x.folder == "Miei"));
    assert_eq!(import_text("niente qui", "").unwrap(), vec![]);
    assert_eq!(
        import_text("https://x.example/f", "  ").unwrap()[0].folder,
        DEFAULT_FOLDER
    );
}

#[tokio::test]
async fn add_feed_normalizes_discovers_and_reports_errors() {
    let (l, base) = bind().await;
    let mut r = HashMap::new();
    r.insert(
        "/".into(),
        route(
            "text/html",
            r#"<html><head><link rel="alternate" type="application/rss+xml" href="/rss.xml"></head></html>"#,
        ),
    );
    r.insert("/rss.xml".into(), route("application/rss+xml", rss_utf8()));
    serve(l, r);
    let client = build_client(Duration::from_secs(5)).unwrap();

    let ok = add_feed(&client, &format!("  {base}/  ")).await;
    assert!(ok.ok, "{:?}", ok.error);
    assert_eq!(ok.url, format!("{base}/"));
    assert_eq!(ok.feed.unwrap().url, format!("{base}/rss.xml"));

    let missing = add_feed(&client, &format!("{base}/nope")).await;
    assert!(!missing.ok);
    assert_eq!(missing.error.as_deref(), Some("il server ha risposto 404"));

    let empty = add_feed(&client, "   ").await;
    assert_eq!(empty.error.as_deref(), Some("inserisci un indirizzo"));
}

#[test]
fn parse_local_uses_file_scheme_key() {
    let f = parse_local("backup feed.xml", rss_utf8().as_bytes()).unwrap();
    assert_eq!(f.url, "file:backup feed.xml");
    assert_eq!(f.articles.len(), 3);
}
