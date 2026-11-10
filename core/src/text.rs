use scraper::{Html, Node, Selector};

const BLOCK_TAGS: &[&str] = &[
    "p",
    "div",
    "br",
    "li",
    "ul",
    "ol",
    "tr",
    "table",
    "blockquote",
    "pre",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "section",
    "article",
    "figure",
    "figcaption",
    "hr",
];

/// Converte HTML in testo semplice: salta script/style, separa i blocchi, comprime gli spazi.
pub fn html_to_text(html: &str) -> String {
    if !html.contains('<') && !html.contains('&') {
        return collapse(html);
    }
    let frag = Html::parse_fragment(html);
    let mut out = String::with_capacity(html.len() / 2);
    for node in frag.root_element().descendants() {
        match node.value() {
            Node::Text(t) => {
                let in_hidden = node
                    .parent()
                    .and_then(|p| p.value().as_element())
                    .map(|e| matches!(e.name(), "script" | "style" | "noscript"))
                    .unwrap_or(false);
                if !in_hidden {
                    out.push_str(t);
                }
            }
            Node::Element(e) if BLOCK_TAGS.contains(&e.name()) => out.push(' '),
            _ => {}
        }
    }
    collapse(&out)
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Taglia a `max` caratteri (non byte) aggiungendo "…".
pub fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max).collect();
    format!("{}…", cut.trim_end())
}

/// Primo `<img src="http…">` dell'HTML, se c'è.
pub fn first_image(html: &str) -> Option<String> {
    if !html.contains("<img") && !html.contains("<IMG") {
        return None;
    }
    let sel = Selector::parse("img").ok()?;
    let frag = Html::parse_fragment(html);
    let found = frag
        .select(&sel)
        .filter_map(|i| i.value().attr("src"))
        .find(|s| s.starts_with("http://") || s.starts_with("https://"))
        .map(str::to_string);
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_markup_scripts_and_entities() {
        let t = html_to_text(
            "<p>Ciao <b>mondo</b></p><style>p{}</style><p>Fine&nbsp;&amp;&egrave;</p>",
        );
        assert_eq!(t, "Ciao mondo Fine &è");
    }

    #[test]
    fn plain_text_passes_through() {
        assert_eq!(html_to_text("  a \n b  "), "a b");
        assert_eq!(html_to_text(""), "");
    }

    #[test]
    fn truncates_on_char_boundaries() {
        assert_eq!(truncate_chars("èèèèè", 3), "èèè…");
        assert_eq!(truncate_chars("ab", 5), "ab");
    }

    #[test]
    fn finds_first_http_image() {
        assert_eq!(
            first_image(r#"<p><img src="data:x"><img src="https://x/a.png"></p>"#).as_deref(),
            Some("https://x/a.png")
        );
        assert_eq!(first_image("<p>niente</p>"), None);
    }
}
