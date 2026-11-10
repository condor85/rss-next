use scraper::{Html, Selector};
use url::Url;

/// Un feed trovato in una pagina web.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub url: String,
    pub title: Option<String>,
}

const FEED_TYPES: &[&str] = &[
    "application/rss+xml",
    "application/atom+xml",
    "application/feed+json",
    "application/json",
    "application/xml",
    "text/xml",
];

/// Cerca `<link rel="alternate" type="application/rss+xml" href="…">` in una pagina HTML.
pub fn discover_feeds(html: &str, base: &str) -> Vec<Candidate> {
    let Ok(base) = Url::parse(base) else {
        return vec![];
    };
    let Ok(sel) = Selector::parse("link[rel][href]") else {
        return vec![];
    };
    let doc = Html::parse_document(html);
    let mut out: Vec<Candidate> = vec![];
    for l in doc.select(&sel) {
        let v = l.value();
        let rel = v.attr("rel").unwrap_or("").to_ascii_lowercase();
        if !rel.split_whitespace().any(|r| r == "alternate") {
            continue;
        }
        let ty = v.attr("type").unwrap_or("").to_ascii_lowercase();
        if !FEED_TYPES.contains(&ty.as_str()) {
            continue;
        }
        let Some(href) = v.attr("href") else { continue };
        let Ok(abs) = base.join(href.trim()) else {
            continue;
        };
        if !matches!(abs.scheme(), "http" | "https") {
            continue;
        }
        let url = abs.to_string();
        if out.iter().any(|c| c.url == url) {
            continue;
        }
        out.push(Candidate {
            url,
            title: v
                .attr("title")
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty()),
        });
    }
    out
}

/// Percorsi usati comunemente quando la pagina non dichiara nessun feed.
pub fn common_feed_urls(base: &str) -> Vec<String> {
    let Ok(base) = Url::parse(base) else {
        return vec![];
    };
    [
        "/feed",
        "/rss",
        "/feed.xml",
        "/rss.xml",
        "/atom.xml",
        "/index.xml",
    ]
    .iter()
    .filter_map(|p| base.join(p).ok())
    .map(|u| u.to_string())
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_relative_and_skips_non_feeds() {
        let html = r#"<head>
          <link rel="alternate" type="application/atom+xml" href="feed/atom" title=" Atom ">
          <link rel="alternate" type="text/html" href="/it/">
          <link rel="ALTERNATE stylesheet" type="application/rss+xml" href="https://x.example/rss">
          <link rel="alternate" type="application/rss+xml" href="javascript:void(0)">
          <link rel="alternate" type="application/atom+xml" href="feed/atom">
        </head>"#;
        let c = discover_feeds(html, "https://x.example/blog/");
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].url, "https://x.example/blog/feed/atom");
        assert_eq!(c[0].title.as_deref(), Some("Atom"));
        assert_eq!(c[1].url, "https://x.example/rss");
    }

    #[test]
    fn common_paths_are_absolute() {
        let u = common_feed_urls("https://x.example/blog/post");
        assert_eq!(u[0], "https://x.example/feed");
        assert_eq!(u.len(), 6);
    }
}
