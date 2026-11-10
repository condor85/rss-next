# rss-next

Aggregatore di notizie open source per Android, creato con Claude. Licenza GPL-3.0.

App Android (e desktop) per leggere feed RSS/Atom/JSON Feed, scritta in **Rust** con **Tauri 2**.

- Download dei feed **direttamente da Rust** (rustls): niente proxy, niente limiti CORS, tutti i feed in parallelo.
- Un feed rotto, lento o enorme **non blocca mai** gli altri: ognuno ha il suo esito e il suo errore leggibile.
- Cartelle, contatori di non letti, filtro per feed, "solo non letti", miniature, tema chiaro/scuro.
- Aggiungi un feed dall'indirizzo del feed **o del sito** (se è una pagina web, il feed viene cercato da solo).
- Importa **OPML** (le cartelle restano), elenchi di URL, file RSS/Atom; esporta in OPML.
- Aggiornamento automatico con l'app aperta e notifiche per i nuovi articoli.

## Struttura

| Cartella | Contenuto |
|---|---|
| `core/` | Nucleo Rust puro (download, parsing, OPML, scoperta feed) + 25 test. Non dipende da Tauri. |
| `src-tauri/` | Guscio Tauri: espone i comandi del nucleo all'interfaccia. |
| `ui/` | Interfaccia (HTML/CSS/JS senza dipendenze). Aprendo `ui/index.html` in un browser parte un ripiego JS con più limiti di rete. |

## Provare il nucleo

```
cargo test            # esegue i test del nucleo (server HTTP locale, nessuna rete esterna)
```

## Compilare l'APK

### Opzione A — GitHub Actions (consigliata, non installi nulla)
1. Crea un repository GitHub e carica questa cartella.
2. Scheda **Actions → APK Android → Run workflow**.
3. A fine lavoro scarica `lettore-rss-apk` dagli *Artifacts*, copia l'APK sul telefono e aprilo
   (Android chiederà di consentire l'installazione da quella fonte).

### Opzione B — sul tuo PC
Servono: Rust, JDK 17, Android SDK + NDK 27, `cargo install tauri-cli --version "^2"`.
```
rustup target add aarch64-linux-android
cd src-tauri
cargo tauri android init
cargo tauri android build --apk --debug --target aarch64
```
Su Bazzite conviene farlo in un distrobox (es. Ubuntu) con `JAVA_HOME`, `ANDROID_HOME` e `NDK_HOME` impostati.

### Versione desktop (Linux)
```
cd src-tauri && cargo tauri dev
```
Richiede `libwebkit2gtk-4.1-dev` e le dipendenze GTK (in distrobox Fedora: `webkit2gtk4.1-devel`).

## Limiti noti
- **Notifiche con app chiusa:** Android non permette controlli periodici a un'app in background senza un servizio dedicato
  (WorkManager/foreground service) o un server push. Le notifiche arrivano quando l'app è aperta o in background.
- L'APK prodotto dal workflow è firmato con la chiave di debug: va bene per uso personale, non per il Play Store.
