use thiserror::Error;

/// Errori del nucleo. I messaggi sono in italiano perché vengono mostrati all'utente.
#[derive(Error, Debug, Clone, PartialEq)]
pub enum Error {
    #[error("inserisci un indirizzo")]
    Empty,
    #[error("URL non valido: {0}")]
    BadUrl(String),
    #[error("rete: {0}")]
    Network(String),
    #[error("il server ha risposto {0}")]
    Status(u16),
    #[error("risposta troppo grande")]
    TooLarge,
    #[error("non è un feed RSS/Atom valido")]
    NotAFeed,
    #[error("OPML non valido: {0}")]
    Opml(String),
}

impl Error {
    /// Vale la pena riprovare una volta (timeout, errori di rete, 5xx)?
    pub fn is_transient(&self) -> bool {
        matches!(self, Error::Network(_)) || matches!(self, Error::Status(s) if *s >= 500)
    }
}

pub(crate) fn net(e: reqwest::Error) -> Error {
    if e.is_timeout() {
        Error::Network("tempo scaduto".into())
    } else if e.is_connect() {
        Error::Network("connessione non riuscita".into())
    } else {
        Error::Network(e.without_url().to_string())
    }
}
