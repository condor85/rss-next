//! Nucleo del lettore RSS. Non dipende da Tauri né da Android: si compila e si prova ovunque.
//!
//! - [`fetch_feed`] / [`refresh_all`]: download + parsing (RSS, Atom, JSON Feed), con scoperta
//!   del feed a partire da una pagina web.
//! - [`parse_feed`]: solo parsing, da byte già in memoria.
//! - [`import_text`] / [`export_opml`]: OPML con cartelle ed elenchi di URL.

pub mod discover;
pub mod error;
pub mod fetch;
pub mod media;
pub mod model;
pub mod opml;
pub mod parse;
pub mod text;

pub use error::Error;
pub use fetch::{add_feed, build_client, fetch_feed, normalize_url, refresh_all};
pub use model::{Article, FeedData, FeedOutcome, ImportedFeed};
pub use opml::{export_opml, extract_urls, import_text, parse_opml, DEFAULT_FOLDER};
pub use parse::{parse_feed, parse_local};
pub use reqwest::Client;
