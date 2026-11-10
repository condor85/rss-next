use crate::error::Error;
use crate::model::ImportedFeed;
use opml::{Outline, OPML};

pub const DEFAULT_FOLDER: &str = "Generale";

fn walk(outlines: &[Outline], folder: &str, out: &mut Vec<ImportedFeed>) {
    for o in outlines {
        let name = o
            .title
            .clone()
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| o.text.clone());
        match &o.xml_url {
            Some(u) if !u.trim().is_empty() => out.push(ImportedFeed {
                url: u.trim().to_string(),
                title: Some(name.trim().to_string()).filter(|t| !t.is_empty()),
                folder: folder.to_string(),
            }),
            _ => {
                // Un outline senza xmlUrl è una cartella: i suoi figli ne ereditano il nome.
                let f = if name.trim().is_empty() {
                    folder
                } else {
                    name.trim()
                };
                walk(&o.outlines, f, out);
            }
        }
    }
}

/// Legge un file OPML mantenendo le cartelle.
pub fn parse_opml(xml: &str) -> Result<Vec<ImportedFeed>, Error> {
    let doc = OPML::from_str(xml).map_err(|e| Error::Opml(e.to_string()))?;
    let mut out = vec![];
    walk(&doc.body.outlines, DEFAULT_FOLDER, &mut out);
    Ok(out)
}

/// Estrae gli URL http(s) da un testo qualunque (uno per riga, separati da spazi, …).
pub fn extract_urls(text: &str) -> Vec<String> {
    let mut seen: Vec<String> = vec![];
    for tok in
        text.split(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ',' | ';'))
    {
        let tok = tok
            .trim_start_matches(['(', '['])
            .trim_end_matches(['.', ')', ']']);
        let is_http = tok.starts_with("http://") || tok.starts_with("https://");
        if is_http && url::Url::parse(tok).is_ok() && !seen.iter().any(|s| s == tok) {
            seen.push(tok.to_string());
        }
    }
    seen
}

/// Importa un file di testo: OPML (con cartelle) oppure un elenco di URL.
pub fn import_text(text: &str, folder: &str) -> Result<Vec<ImportedFeed>, Error> {
    let folder = if folder.trim().is_empty() {
        DEFAULT_FOLDER
    } else {
        folder.trim()
    };
    if text.contains("<opml") || text.contains("<OPML") {
        return parse_opml(text);
    }
    Ok(extract_urls(text)
        .into_iter()
        .map(|url| ImportedFeed {
            url,
            title: None,
            folder: folder.to_string(),
        })
        .collect())
}

/// Esporta i feed in OPML, una cartella (outline) per gruppo.
pub fn export_opml(feeds: &[ImportedFeed]) -> Result<String, Error> {
    let mut doc = OPML {
        head: Some(opml::Head {
            title: Some("Lettore RSS".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut folders: Vec<&str> = vec![];
    for f in feeds {
        if !folders.contains(&f.folder.as_str()) {
            folders.push(&f.folder);
        }
    }
    for folder in folders {
        let mut group = Outline {
            text: folder.to_string(),
            ..Default::default()
        };
        for f in feeds.iter().filter(|f| f.folder == folder) {
            group.add_feed(f.title.as_deref().unwrap_or(&f.url), &f.url);
        }
        doc.body.outlines.push(group);
    }
    doc.to_string().map_err(|e| Error::Opml(e.to_string()))
}
