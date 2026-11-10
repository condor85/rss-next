//! `<media:thumbnail url="…"/>` direttamente dentro un `<item>` (lo usano BBC e molte altre testate)
//! viene ignorato da feed-rs, che lo legge solo dentro `<media:content>`/`<media:group>`.
//! Questa piccola scansione lo recupera, associandolo all'articolo tramite guid/id o link.

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};
use std::collections::HashMap;

fn attr(e: &BytesStart, name: &str) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().into_inner() == name)
        .and_then(|a| {
            a.normalized_value(XmlVersion::Implicit1_0)
                .ok()
                .map(|v| v.trim().to_string())
        })
        .filter(|v| !v.is_empty())
}

/// Risolve `&amp;`, `&#38;`, `&#x26;` … (i riferimenti arrivano come eventi separati dal testo).
fn resolve_ref(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        n => {
            let n = n.strip_prefix('#')?;
            let code = match n.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => n.parse().ok()?,
            };
            char::from_u32(code)
        }
    }
}

/// Mappa `guid | id | link` → URL della miniatura, per gli articoli che ne hanno una.
pub fn bare_thumbnails(xml: &[u8]) -> HashMap<String, String> {
    // Gli URL e gli id sono ASCII: una lettura "lossy" va bene anche per feed in ISO-8859-1.
    let text = String::from_utf8_lossy(xml);
    let mut reader = Reader::from_str(&text);
    reader.config_mut().check_end_names = false;

    let mut map = HashMap::new();
    let mut in_item = false;
    let mut capture = false;
    let mut buf = String::new();
    let mut keys: Vec<String> = vec![];
    let mut thumb: Option<String> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => match e.local_name().into_inner() {
                "item" | "entry" => {
                    in_item = true;
                    keys.clear();
                    thumb = None;
                }
                "thumbnail" if in_item => {
                    if thumb.is_none() {
                        thumb = attr(&e, "url");
                    }
                }
                "link" if in_item => match attr(&e, "href") {
                    Some(h) => keys.push(h),
                    None => {
                        capture = true;
                        buf.clear();
                    }
                },
                "guid" | "id" if in_item => {
                    capture = true;
                    buf.clear();
                }
                _ => {}
            },
            Ok(Event::Text(t)) if capture => buf.push_str(&t.xml10_content()),
            Ok(Event::CData(t)) if capture => buf.push_str(&t.xml10_content()),
            Ok(Event::GeneralRef(r)) if capture => {
                if let Some(c) = resolve_ref(&r.xml10_content()) {
                    buf.push(c);
                }
            }
            Ok(Event::End(e)) => match e.local_name().into_inner() {
                "item" | "entry" => {
                    if let Some(u) = thumb.take() {
                        for k in keys.drain(..) {
                            map.entry(k).or_insert_with(|| u.clone());
                        }
                    }
                    in_item = false;
                    capture = false;
                }
                "link" | "guid" | "id" => {
                    if capture && !buf.trim().is_empty() {
                        keys.push(buf.trim().to_string());
                    }
                    capture = false;
                }
                _ => {}
            },
            Ok(Event::Eof) | Err(_) => break,
            _ => {}
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_bare_thumbnails_in_rss_and_atom() {
        let rss = r#"<rss xmlns:media="http://search.yahoo.com/mrss/"><channel><title>t</title>
          <item><title>a</title><link>https://n.example/a</link><guid isPermaLink="false">ga</guid>
            <media:thumbnail width="240" url="https://n.example/i/a.jpg?x=1&amp;y=2"/></item>
          <item><title>b</title><link>https://n.example/b</link></item>
        </channel></rss>"#;
        let m = bare_thumbnails(rss.as_bytes());
        assert_eq!(
            m.get("ga").map(String::as_str),
            Some("https://n.example/i/a.jpg?x=1&y=2")
        );
        assert_eq!(
            m.get("https://n.example/a").map(String::as_str),
            Some("https://n.example/i/a.jpg?x=1&y=2")
        );
        assert!(!m.contains_key("https://n.example/b"));

        let atom = r#"<feed xmlns="http://www.w3.org/2005/Atom" xmlns:media="http://search.yahoo.com/mrss/">
          <entry><id>urn:1</id><link href="https://a.example/1"/><media:thumbnail url="https://a.example/t.png"/></entry></feed>"#;
        let m = bare_thumbnails(atom.as_bytes());
        assert_eq!(
            m.get("urn:1").map(String::as_str),
            Some("https://a.example/t.png")
        );
        assert_eq!(
            m.get("https://a.example/1").map(String::as_str),
            Some("https://a.example/t.png")
        );
    }

    #[test]
    fn keys_with_entities_match() {
        let rss = r#"<rss xmlns:media="http://search.yahoo.com/mrss/"><channel>
          <item><link>https://n.example/a?x=1&amp;y=2</link><media:thumbnail url="https://n.example/t.jpg"/></item>
        </channel></rss>"#;
        let m = bare_thumbnails(rss.as_bytes());
        assert!(m.contains_key("https://n.example/a?x=1&y=2"), "{m:?}");
    }

    #[test]
    fn survives_garbage_and_non_utf8() {
        assert!(bare_thumbnails(b"<<<").is_empty());
        assert!(bare_thumbnails(&[0xE8, 0xFF, b'<', b'x']).is_empty());
    }
}
