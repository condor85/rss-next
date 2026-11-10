//! Guscio Tauri: espone alla pagina web i comandi del nucleo `rss_core`.
//! Tutta la rete passa da qui (Rust + rustls), quindi nessun limite CORS e nessun proxy.

use rss_core::{FeedData, FeedOutcome, ImportedFeed};
use std::time::Duration;
use tauri::State;

struct Http(rss_core::Client);

#[tauri::command]
async fn refresh_feeds(
    http: State<'_, Http>,
    urls: Vec<String>,
) -> Result<Vec<FeedOutcome>, String> {
    Ok(rss_core::refresh_all(&http.0, urls, 8).await)
}

#[tauri::command]
async fn add_feed(http: State<'_, Http>, input: String) -> Result<FeedOutcome, String> {
    Ok(rss_core::add_feed(&http.0, &input).await)
}

#[tauri::command]
fn import_text(text: String, folder: String) -> Result<Vec<ImportedFeed>, String> {
    rss_core::import_text(&text, &folder).map_err(|e| e.to_string())
}

#[tauri::command]
fn parse_local(name: String, text: String) -> Result<FeedData, String> {
    rss_core::parse_local(&name, text.as_bytes()).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_opml(feeds: Vec<ImportedFeed>) -> Result<String, String> {
    rss_core::export_opml(&feeds).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let client = rss_core::build_client(Duration::from_secs(20)).expect("client HTTP");
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(Http(client))
        .invoke_handler(tauri::generate_handler![
            refresh_feeds,
            add_feed,
            import_text,
            parse_local,
            export_opml
        ])
        .run(tauri::generate_context!())
        .expect("errore nell'avvio dell'app");
}
