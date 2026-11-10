use serde::{Deserialize, Serialize};

/// Un articolo, già ripulito dall'HTML e pronto per essere mostrato.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Article {
    pub id: String,
    pub title: String,
    pub link: String,
    /// Millisecondi dall'epoca Unix; 0 se il feed non indica la data.
    pub date: i64,
    /// Testo semplice, al massimo ~600 caratteri.
    pub summary: String,
    pub image: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedData {
    /// URL reale del feed (può differire da quello inserito se è stato "scoperto" da una pagina web).
    pub url: String,
    pub title: String,
    pub site: Option<String>,
    pub articles: Vec<Article>,
}

/// Risultato dell'aggiornamento di un singolo feed: un feed rotto non blocca mai gli altri.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedOutcome {
    /// L'URL richiesto (la chiave con cui l'interfaccia riconosce il feed).
    pub url: String,
    pub ok: bool,
    pub error: Option<String>,
    pub feed: Option<FeedData>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedFeed {
    pub url: String,
    pub title: Option<String>,
    pub folder: String,
}
