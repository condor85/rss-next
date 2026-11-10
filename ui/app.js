'use strict';
/* Lettore RSS — interfaccia. Con Tauri usa il nucleo Rust (comandi `invoke`);
   in un browser normale usa un ripiego in JavaScript (stessa interfaccia, più limiti di rete). */

const $ = (s) => document.querySelector(s);
const DEF = 'Generale';
const PAGE = 50;
const CHUNK = 10;
const MAX_ITEMS = 1200;

const ICONS = {
  back: '<svg viewBox="0 0 24 24"><path d="M20 11H7.8l5.6-5.6L12 4l-8 8 8 8 1.4-1.4L7.8 13H20z"/></svg>',
  refresh: '<svg viewBox="0 0 24 24"><path d="M17.65 6.35A7.96 7.96 0 0 0 12 4a8 8 0 1 0 7.74 10h-2.08A6 6 0 1 1 12 6c1.66 0 3.14.69 4.22 1.78L13 11h7V4z"/></svg>',
  menu: '<svg viewBox="0 0 24 24"><path d="M19.4 13a7.5 7.5 0 0 0 0-2l2.1-1.6-2-3.5-2.5 1a7.4 7.4 0 0 0-1.7-1l-.4-2.7h-4l-.4 2.7a7.4 7.4 0 0 0-1.7 1l-2.5-1-2 3.5L4.6 11a7.5 7.5 0 0 0 0 2l-2.1 1.6 2 3.5 2.5-1c.5.4 1.1.7 1.7 1l.4 2.7h4l.4-2.7c.6-.3 1.2-.6 1.7-1l2.5 1 2-3.5zM12 15.5A3.5 3.5 0 1 1 12 8.5a3.5 3.5 0 0 1 0 7z"/></svg>',
  close: '<svg viewBox="0 0 24 24"><path d="M19 6.4 17.6 5 12 10.6 6.4 5 5 6.4 10.6 12 5 17.6 6.4 19 12 13.4l5.6 5.6 1.4-1.4-5.6-5.6z"/></svg>',
  folder: '<svg viewBox="0 0 24 24"><path d="M10 4H4a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-8z"/></svg>',
  all: '<svg viewBox="0 0 24 24"><path d="M4 5h16v2H4zm0 4h16v2H4zm0 4h10v2H4zm0 4h10v2H4zm12-4h4v6h-4z"/></svg>',
  edit: '<svg viewBox="0 0 24 24"><path d="M3 17.25V21h3.75L17.8 9.94l-3.75-3.75zM20.7 7.04a1 1 0 0 0 0-1.41l-2.34-2.34a1 1 0 0 0-1.41 0l-1.83 1.83 3.75 3.75z"/></svg>',
  trash: '<svg viewBox="0 0 24 24"><path d="M6 19a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2V7H6zM19 4h-3.5l-1-1h-5l-1 1H5v2h14z"/></svg>',
  rss: '<svg viewBox="0 0 24 24"><circle cx="6.2" cy="17.8" r="2.2"/><path d="M4 10.1v3a7 7 0 0 1 7 7h3a10 10 0 0 0-10-10zm0-6v3a13 13 0 0 1 13 13h3A16 16 0 0 0 4 4.1z"/></svg>',
};

/* ---------------------------------------------------------------- stato */
let store = { feeds: [], items: [], read: {}, lastRef: 0, settings: { auto: 15, images: true, unreadOnly: true, notify: false } };
let view = { folder: null, feed: null, sheet: false };
let limit = PAGE;
let busy = false;

function load() {
  try {
    const s = JSON.parse(localStorage.getItem('rss3') || 'null');
    if (s) {
      store = { ...store, ...s, settings: { ...store.settings, ...(s.settings || {}) } };
    }
  } catch (_) {}
  store.feeds.forEach((f) => { if (!f.folder) f.folder = DEF; });
}
let saveTimer;
function save() {
  clearTimeout(saveTimer);
  saveTimer = setTimeout(saveNow, 150);
}
function saveNow() {
  try { localStorage.setItem('rss3', JSON.stringify(store)); } catch (_) {}
}
addEventListener('pagehide', saveNow);
document.addEventListener('visibilitychange', () => { if (document.hidden) saveNow(); });

/* ---------------------------------------------------------------- utilità */
function el(tag, cls, text) {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text != null) e.textContent = text;
  return e;
}
function icon(name) { const s = document.createElement('span'); s.style.display = 'contents'; s.innerHTML = ICONS[name]; return s; }
function setIcon(node, name) { node.innerHTML = ICONS[name]; }
let toastT;
function toast(t) {
  const e = $('#toast');
  e.textContent = t;
  e.classList.add('show');
  clearTimeout(toastT);
  toastT = setTimeout(() => e.classList.remove('show'), 3200);
}
const keyOf = (i) => i.feed + '|' + i.id;
const isRead = (i) => !!store.read[keyOf(i)];
const unread = (list) => list.filter((i) => !isRead(i)).length;
const feedByUrl = (u) => store.feeds.find((f) => f.url === u);
function folders() {
  return [...new Set(store.feeds.map((f) => f.folder))].sort((a, b) => a.localeCompare(b, 'it'));
}
function feedsOf(folder) { return store.feeds.filter((f) => folder === '*' || f.folder === folder); }
function itemsOf(folder, feed) {
  const urls = new Set(feedsOf(folder).map((f) => f.url));
  return store.items.filter((i) => urls.has(i.feed) && (!feed || i.feed === feed));
}
function when(ms) {
  if (!ms) return '';
  const d = Date.now() - ms;
  if (d < 0) return 'ora';
  const m = Math.floor(d / 60000);
  if (m < 1) return 'ora';
  if (m < 60) return m + ' min fa';
  const h = Math.floor(m / 60);
  if (h < 24) return h + (h === 1 ? ' ora fa' : ' ore fa');
  const g = Math.floor(h / 24);
  if (g === 1) return 'ieri';
  if (g < 7) return g + ' giorni fa';
  return new Date(ms).toLocaleDateString('it-IT', { day: 'numeric', month: 'short', year: new Date(ms).getFullYear() === new Date().getFullYear() ? undefined : 'numeric' });
}
function safeHref(u) { return /^https?:\/\//i.test(u || '') ? u : ''; }

/* dialogo al posto di prompt()/confirm(), che nelle WebView non sono affidabili */
function ask({ title, text, input, value, ok }) {
  return new Promise((resolve) => {
    const d = $('#dlg'), inp = $('#dlgIn'), p = $('#dlgP');
    $('#dlgT').textContent = title;
    p.hidden = !text; p.textContent = text || '';
    inp.hidden = !input; inp.value = value || '';
    $('#dlgOk').textContent = ok || 'OK';
    d.onclose = () => resolve(d.returnValue === 'ok' ? (input ? inp.value.trim() : true) : null);
    d.returnValue = 'cancel';
    d.showModal();
    if (input) setTimeout(() => inp.focus(), 50);
  });
}

/* ---------------------------------------------------------------- backend */
const T = window.__TAURI__;
const invoke = T && T.core && T.core.invoke;
const native = !!invoke;

const backendNative = {
  refresh: (urls) => invoke('refresh_feeds', { urls }),
  add: (input) => invoke('add_feed', { input }),
  importText: (text, folder) => invoke('import_text', { text, folder }),
  parseLocal: (name, text) => invoke('parse_local', { name, text }),
  exportOpml: (feeds) => invoke('export_opml', { feeds }),
};

/* ripiego per il browser: stessa forma dei dati del nucleo Rust */
const backendWeb = (() => {
  const proxies = [
    (u) => u,
    (u) => 'https://corsproxy.io/?url=' + encodeURIComponent(u),
    (u) => 'https://api.allorigins.win/raw?url=' + encodeURIComponent(u),
  ];
  const text = (h) => { const d = new DOMParser().parseFromString(h || '', 'text/html'); return (d.body.textContent || '').replace(/\s+/g, ' ').trim(); };
  const cut = (s, n) => (s.length > n ? s.slice(0, n).trimEnd() + '…' : s);
  async function get(url) {
    for (const p of proxies) {
      try {
        const c = new AbortController(); const t = setTimeout(() => c.abort(), 12000);
        const r = await fetch(p(url), { signal: c.signal }); clearTimeout(t);
        if (r.ok) return await r.text();
      } catch (_) {}
    }
    throw new Error('rete: non raggiungibile (nel browser molti siti bloccano la lettura)');
  }
  function parse(url, xml) {
    const d = new DOMParser().parseFromString(xml, 'text/xml');
    if (d.querySelector('parsererror') || !d.querySelector('channel, feed')) throw new Error('non è un feed RSS/Atom valido');
    const first = (n, names) => { for (const c of n.children) if (names.includes(c.localName)) return c; return null; };
    const t = d.querySelector('channel > title, feed > title');
    const items = [...d.querySelectorAll('item, entry')].map((n) => {
      const g = (...names) => { const e = first(n, names); return e ? e.textContent.trim() : ''; };
      let link = g('link');
      const ls = [...n.children].filter((c) => c.localName === 'link');
      if (ls.length) { const a = ls.find((l) => !l.getAttribute('rel') || l.getAttribute('rel') === 'alternate') || ls[0]; link = a.getAttribute('href') || a.textContent.trim(); }
      const html = g('encoded', 'content') || g('description', 'summary');
      const img = (html.match(/<img[^>]+src=["'](https?:[^"']+)/i) || [])[1];
      const th = [...n.children].find((c) => c.localName === 'thumbnail' || (c.localName === 'content' && c.getAttribute('url')));
      return {
        id: g('guid', 'id') || link, title: text(g('title')) || '(senza titolo)', link,
        date: Date.parse(g('pubDate', 'published', 'updated', 'date')) || 0,
        summary: cut(text(g('description', 'summary') || g('encoded', 'content')), 600),
        image: (th && th.getAttribute('url')) || img || null,
      };
    }).sort((a, b) => b.date - a.date).slice(0, 100);
    return { url, title: t ? text(t.textContent) : url, site: null, articles: items };
  }
  async function fetchFeed(url) {
    const body = await get(url);
    try { return parse(url, body); } catch (e) {
      if (!/<html/i.test(body.slice(0, 4000))) throw e;
      const doc = new DOMParser().parseFromString(body, 'text/html');
      const cands = [...doc.querySelectorAll('link[rel~=alternate][href]')]
        .filter((l) => /rss|atom|xml|json/i.test(l.getAttribute('type') || ''))
        .map((l) => { try { return new URL(l.getAttribute('href'), url).href; } catch (_) { return null; } }).filter(Boolean);
      for (const c of cands.slice(0, 5)) { try { return parse(c, await get(c)); } catch (_) {} }
      throw e;
    }
  }
  const outcome = async (url) => { try { return { url, ok: true, error: null, feed: await fetchFeed(url) }; } catch (e) { return { url, ok: false, error: e.message, feed: null }; } };
  const urlsIn = (t) => [...new Set((t.match(/https?:\/\/[^\s"'<>,;]+/g) || []).map((u) => u.replace(/[.)\]]+$/, '')))];
  return {
    refresh: (urls) => Promise.all(urls.map(outcome)),
    add: async (input) => {
      const t = input.trim();
      if (!t) return { url: '', ok: false, error: 'inserisci un indirizzo', feed: null };
      return outcome(/^https?:\/\//i.test(t) ? t : 'https://' + t);
    },
    importText: async (txt, folder) => {
      folder = (folder || '').trim() || DEF;
      if (/<opml/i.test(txt)) {
        const d = new DOMParser().parseFromString(txt, 'text/xml'), out = [];
        const walk = (n, f) => [...n.children].forEach((o) => {
          if (o.localName !== 'outline') return;
          const name = (o.getAttribute('title') || o.getAttribute('text') || '').trim(), x = o.getAttribute('xmlUrl');
          if (x) out.push({ url: x.trim(), title: name || null, folder: f }); else walk(o, name || f);
        });
        const b = d.querySelector('body'); if (b) walk(b, DEF);
        return out;
      }
      return urlsIn(txt).map((url) => ({ url, title: null, folder }));
    },
    parseLocal: async (name, txt) => parse('file:' + name, txt),
    exportOpml: async (feeds) => {
      const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/"/g, '&quot;');
      const by = {}; feeds.forEach((f) => (by[f.folder] = by[f.folder] || []).push(f));
      return '<?xml version="1.0" encoding="UTF-8"?>\n<opml version="2.0"><head><title>Lettore RSS</title></head><body>' +
        Object.entries(by).map(([k, v]) => `<outline text="${esc(k)}">` + v.map((f) => `<outline type="rss" text="${esc(f.title || f.url)}" xmlUrl="${esc(f.url)}"/>`).join('') + '</outline>').join('') + '</body></opml>';
    },
  };
})();

const backend = native ? backendNative : backendWeb;

/* ---------------------------------------------------------------- dati */
function mergeArticles(feedUrl, articles, markNew) {
  const known = new Map(store.items.map((i) => [keyOf(i), i]));
  let fresh = 0;
  for (const a of articles) {
    const it = { feed: feedUrl, id: a.id, title: a.title, link: a.link, date: a.date, summary: a.summary, image: a.image || null };
    const k = keyOf(it);
    const old = known.get(k);
    if (!old) { fresh++; it.fresh = markNew ? 1 : 0; } else it.fresh = old.fresh;
    known.set(k, it);
  }
  store.items = [...known.values()].sort((a, b) => b.date - a.date).slice(0, MAX_ITEMS);
  return fresh;
}
function pruneRead() {
  const keys = new Set(store.items.map(keyOf));
  for (const k of Object.keys(store.read)) if (!keys.has(k)) delete store.read[k];
}

function applyOutcome(o, markNew) {
  const f = feedByUrl(o.url);
  if (!f) return 0;
  f.status = { ok: o.ok, error: o.error, at: Date.now() };
  if (!o.ok) return 0;
  // Se l'indirizzo era una pagina web, il nucleo ha trovato il feed vero: lo usiamo da ora in poi.
  if (o.feed.url !== f.url && !feedByUrl(o.feed.url)) f.url = o.feed.url;
  if (!f.titleSet) f.title = o.feed.title;
  const had = store.items.some((i) => i.feed === f.url);
  const fresh = mergeArticles(f.url, o.feed.articles, markNew && had);
  // Il primo caricamento di un feed non conta come "novità".
  return markNew && had ? fresh : 0;
}

async function refreshAll({ silent = false } = {}) {
  if (busy) return;
  const feeds = store.feeds.filter((f) => !f.local);
  if (!feeds.length) { render(); return; }
  busy = true;
  $('#bRef').classList.add('spin');
  const bar = $('#progress'), fill = bar.firstElementChild;
  bar.hidden = false; fill.style.width = '4%';
  let added = 0, failed = 0, done = 0;
  try {
    const urls = feeds.map((f) => f.url);
    for (let i = 0; i < urls.length; i += CHUNK) {
      const part = urls.slice(i, i + CHUNK);
      let outs;
      try { outs = await backend.refresh(part); } catch (e) { outs = part.map((u) => ({ url: u, ok: false, error: String(e), feed: null })); }
      for (const o of outs) { added += applyOutcome(o, true); if (!o.ok) failed++; }
      done += part.length;
      fill.style.width = Math.max(4, Math.round((done / urls.length) * 100)) + '%';
      render(); if (view.sheet) renderSheet();
    }
    store.lastRef = Date.now();
    pruneRead();
    saveNow();
  } finally {
    busy = false;
    $('#bRef').classList.remove('spin');
    setTimeout(() => { bar.hidden = true; fill.style.width = '0'; }, 250);
  }
  render(); if (view.sheet) renderSheet();
  if (added) { announce(added); }
  else if (!silent) toast(failed ? `${failed} ${failed === 1 ? 'feed non raggiungibile' : 'feed non raggiungibili'}` : 'Tutto aggiornato');
  else if (failed && !silent) toast(failed + ' feed non raggiungibili');
}

async function announce(n) {
  const msg = n === 1 ? '1 nuovo articolo' : n + ' nuovi articoli';
  if (!document.hidden) { toast(msg); return; }
  if (store.settings.notify && T && T.notification) {
    try { T.notification.sendNotification({ title: 'Lettore RSS', body: msg }); } catch (_) {}
  }
}

/* ---------------------------------------------------------------- aggiunta / import */
function msg(text, kind) {
  const m = $('#addMsg');
  m.hidden = !text; m.textContent = text || ''; m.className = 'msg ' + (kind || '');
}
async function addByInput() {
  const input = $('#inUrl').value;
  const folder = $('#inFolder').value.trim() || DEF;
  const btn = $('#bAdd');
  if (!input.trim()) { msg('Inserisci l’indirizzo di un feed o di un sito.', 'err'); return; }
  btn.disabled = true; btn.textContent = 'Controllo…'; msg('');
  try {
    const o = await backend.add(input);
    if (!o.ok) { msg(o.error || 'Feed non raggiungibile.', 'err'); return; }
    if (feedByUrl(o.feed.url)) { msg('Questo feed c’è già.', 'err'); return; }
    store.feeds.push({ url: o.feed.url, title: o.feed.title, folder });
    applyOutcome({ ...o, url: o.feed.url }, false);
    saveNow();
    $('#inUrl').value = '';
    msg(`Aggiunto: ${o.feed.title} (${o.feed.articles.length} articoli)`, 'ok');
    render(); renderSheet();
  } catch (e) {
    msg(String(e), 'err');
  } finally {
    btn.disabled = false; btn.textContent = 'Aggiungi';
  }
}

async function importFile(file) {
  const txt = await file.text();
  const folder = $('#inFolder').value.trim() || DEF;
  let added = 0;
  try {
    if (/<(rss|feed|rdf)[\s>]/i.test(txt) && !/<opml/i.test(txt)) {
      const feed = await backend.parseLocal(file.name, txt);
      const key = 'file:' + file.name;
      store.feeds = store.feeds.filter((f) => f.url !== key);
      store.items = store.items.filter((i) => i.feed !== key);
      store.feeds.push({ url: key, title: feed.title, folder, local: true, titleSet: false });
      mergeArticles(key, feed.articles, false);
      added = 1;
    } else {
      const list = await backend.importText(txt, folder);
      for (const f of list) {
        if (feedByUrl(f.url)) continue;
        store.feeds.push({ url: f.url, title: f.title || f.url, folder: f.folder || folder, titleSet: !!f.title });
        added++;
      }
    }
  } catch (e) { toast(String(e)); return; }
  saveNow(); render(); renderSheet();
  msg(added ? `Importati ${added} feed` : 'Nessun feed nuovo trovato nel file.', added ? 'ok' : 'err');
  if (added) refreshAll();
}

/* ---------------------------------------------------------------- navigazione */
function go(next, push) {
  view = { folder: null, feed: null, sheet: false, ...next };
  limit = PAGE;
  if (push) history.pushState(view, '');
  render();
  renderSheetVisibility();
  if (!next.keepScroll) scrollTo(0, 0);
}
addEventListener('popstate', (e) => {
  view = e.state ? { folder: null, feed: null, sheet: false, ...e.state } : { folder: null, feed: null, sheet: false };
  limit = PAGE; render(); renderSheetVisibility();
});
function renderSheetVisibility() {
  $('#sheet').hidden = !view.sheet;
  document.body.style.overflow = view.sheet ? 'hidden' : '';
  if (view.sheet) renderSheet();
}
function back() { if (history.state) history.back(); else go({}, false); }

/* ---------------------------------------------------------------- disegno */
function render() {
  const main = $('#main');
  main.replaceChildren();
  const total = unread(store.items);
  document.title = (total ? `(${total}) ` : '') + 'Lettore RSS';
  const home = view.folder === null;
  $('#bBack').hidden = home;
  $('#chips').hidden = home;
  $('#tools').hidden = home;
  $('#ttl').textContent = home ? 'Lettore RSS' : view.folder === '*' ? 'Tutti gli articoli' : view.folder;
  const n = $('#notice');
  n.hidden = native || !store.feeds.length;
  if (!n.hidden) n.textContent = 'Stai usando la versione da browser: molti siti possono bloccare i feed. L’app Android scarica i feed direttamente.';
  home ? renderHome(main, total) : renderList(main);
}

function renderHome(main, total) {
  if (!store.feeds.length) {
    const e = el('div', 'empty');
    e.append(icon('rss'), el('h2', '', 'Nessun feed'), el('div', '', 'Aggiungi un feed o importa un file OPML con le tue fonti.'));
    const b = el('button', 'btn', 'Aggiungi feed');
    b.onclick = () => go({ sheet: true }, true);
    e.append(b);
    main.append(e);
    return;
  }
  const w = el('div', 'wrap');
  const mk = (cls, ico, name, sub, n, onclick) => {
    const t = el('button', 'tile ' + cls);
    const i = el('div', 'ico'); i.append(icon(ico));
    const tx = el('div', 'tx'); tx.append(el('div', 'nm', name)); tx.append(sub);
    t.append(i, tx);
    if (n) t.append(el('span', 'badge', n > 99 ? '99+' : String(n)));
    t.onclick = onclick;
    return t;
  };
  const sub = (feeds) => {
    const s = el('div', 'sub', `${feeds.length} ${feeds.length === 1 ? 'feed' : 'feed'}`);
    const bad = feeds.filter((f) => f.status && !f.status.ok).length;
    if (bad) { const b = el('span', 'bad', ` · ${bad} con errori`); s.append(b); }
    return s;
  };
  w.append(mk('all', 'all', 'Tutti gli articoli', sub(store.feeds), total, () => go({ folder: '*' }, true)));
  for (const fo of folders()) {
    const fs = feedsOf(fo);
    w.append(mk('', 'folder', fo, sub(fs), unread(itemsOf(fo)), () => go({ folder: fo }, true)));
  }
  main.append(w);
}

function renderList(main) {
  const chips = $('#chips');
  chips.replaceChildren();
  const addChip = (label, feed, n) => {
    const c = el('button', 'chip' + (view.feed === feed ? ' on' : ''));
    c.append(el('span', 'n', label));
    if (n) c.append(el('span', 'badge', String(n)));
    c.onclick = () => { view.feed = feed; limit = PAGE; history.replaceState(view, ''); render(); };
    chips.append(c);
  };
  const fs = feedsOf(view.folder);
  if (fs.length > 1) {
    addChip('Tutti', null, unread(itemsOf(view.folder)));
    fs.forEach((f) => addChip(f.title || f.url, f.url, unread(itemsOf(view.folder, f.url))));
  } else chips.hidden = true;

  const only = store.settings.unreadOnly;
  $('#segUnread').classList.toggle('on', only);
  $('#segAll').classList.toggle('on', !only);

  let list = itemsOf(view.folder, view.feed);
  if (only) list = list.filter((i) => !isRead(i));
  if (!list.length) {
    const e = el('div', 'empty');
    const bad = fs.filter((f) => f.status && !f.status.ok);
    e.append(el('h2', '', only ? 'Tutto letto' : 'Nessun articolo'));
    if (bad.length && !itemsOf(view.folder, view.feed).length) e.append(el('div', '', 'Alcuni feed non sono raggiungibili: controlla la lista in ⚙.'));
    else e.append(el('div', '', only ? 'Non ci sono articoli da leggere.' : 'Premi ⟳ per aggiornare.'));
    main.append(e);
    return;
  }
  const names = Object.fromEntries(store.feeds.map((f) => [f.url, f.title || f.url]));
  list.slice(0, limit).forEach((i) => main.append(card(i, names)));
  if (list.length > limit) {
    const b = el('button', 'btn sec more-btn', `Mostra altri (${list.length - limit})`);
    b.onclick = () => { limit += PAGE; render(); };
    main.append(b);
  } else main.append(el('div', 'nomore', 'Hai visto tutto.'));
}

function card(i, names) {
  const k = keyOf(i);
  const c = el('article', 'card' + (isRead(i) ? ' read' : ''));
  const row = el('div', 'row');
  const txt = el('div', 'txt');
  const meta = el('div', 'meta');
  if (i.fresh && !isRead(i)) meta.append(el('span', 'dot'));
  meta.append(el('span', 'src', names[i.feed] || ''), el('span', '', '·'), el('span', '', when(i.date)));
  txt.append(meta, el('h3', '', i.title));
  row.append(txt);
  if (store.settings.images && i.image) {
    const im = new Image();
    im.className = 'th'; im.loading = 'lazy'; im.alt = ''; im.referrerPolicy = 'no-referrer'; im.src = i.image;
    im.onerror = () => im.remove();
    row.append(im);
  }
  const more = el('div', 'more');
  if (i.summary) more.append(el('p', '', i.summary));
  const acts = el('div', 'acts');
  const href = safeHref(i.link);
  if (href) {
    const open = el('button', 'btn', 'Apri articolo');
    open.onclick = (e) => { e.stopPropagation(); markRead(i, c); openLink(href); };
    acts.append(open);
  }
  const tog = el('button', 'btn sec', isRead(i) ? 'Segna da leggere' : 'Segna letto');
  tog.onclick = (e) => {
    e.stopPropagation();
    if (isRead(i)) { delete store.read[k]; } else markRead(i, c);
    save(); render();
  };
  acts.append(tog);
  more.append(acts);
  row.onclick = () => { c.classList.toggle('open'); if (c.classList.contains('open')) markRead(i, c, true); };
  c.append(row, more);
  return c;
}
function markRead(i, c, soft) {
  const k = keyOf(i);
  if (store.read[k]) return;
  store.read[k] = 1; i.fresh = 0; save();
  c.classList.add('read');
  const d = c.querySelector('.dot'); if (d) d.remove();
  refreshBadges();
}
function refreshBadges() {
  // aggiorna solo i contatori senza ridisegnare (mantiene aperta la scheda che si sta leggendo)
  const total = unread(store.items);
  document.title = (total ? `(${total}) ` : '') + 'Lettore RSS';
}
function openLink(u) {
  const op = T && T.opener && T.opener.openUrl;
  if (op) { op(u).catch(() => window.open(u, '_blank')); return; }
  window.open(u, '_blank', 'noopener');
}

/* ---------------------------------------------------------------- gestione */
function renderSheet() {
  const fl = folders();
  const dl = $('#folderList'); dl.replaceChildren();
  fl.forEach((f) => { const o = document.createElement('option'); o.value = f; dl.append(o); });
  const box = $('#feeds'); box.replaceChildren();
  if (!store.feeds.length) { box.append(el('p', 'hint', 'Ancora nessun feed.')); }
  fl.forEach((folder) => {
    const h = el('div', 'fh');
    h.append(icon('folder'), el('b', '', folder));
    const rn = el('button', 'ic'); rn.setAttribute('aria-label', 'Rinomina cartella'); rn.append(icon('edit'));
    rn.onclick = async () => {
      const n = await ask({ title: 'Rinomina cartella', input: true, value: folder, ok: 'Salva' });
      if (n && n !== folder) { store.feeds.forEach((f) => { if (f.folder === folder) f.folder = n; }); if (view.folder === folder) view.folder = n; saveNow(); render(); renderSheet(); }
    };
    h.append(rn); box.append(h);
    feedsOf(folder).forEach((f) => box.append(feedRow(f, fl)));
  });
  $('#setAuto').value = String(store.settings.auto);
  $('#setImages').checked = !!store.settings.images;
  $('#setNotify').checked = !!store.settings.notify;
  const hint = $('#notifyHint');
  hint.hidden = !store.settings.notify;
  hint.textContent = 'Le notifiche arrivano quando l’app ha trovato articoli nuovi mentre è in background. Con l’app chiusa del tutto Android non permette controlli periodici.';
  $('#about').textContent = `${store.feeds.length} feed · ${store.items.length} articoli salvati${native ? '' : ' · versione browser'}`;
}

function feedRow(f, fl) {
  const r = el('div', 'feed');
  const st = el('span', 'st' + (f.status ? (f.status.ok ? ' ok' : ' err') : ''));
  const tx = el('div', 'tx');
  tx.append(el('div', 'nm', f.title || f.url));
  const err = f.status && !f.status.ok;
  const sub = el('div', 'sub' + (err ? ' err' : ''), err ? f.status.error : f.local ? 'File locale' : f.url);
  tx.append(sub);
  const sel = document.createElement('select');
  sel.setAttribute('aria-label', 'Sposta in cartella');
  [...fl, '__new'].forEach((n) => {
    const o = document.createElement('option');
    o.value = n; o.textContent = n === '__new' ? '＋ Nuova…' : n; o.selected = n === f.folder;
    sel.append(o);
  });
  sel.onchange = async () => {
    let n = sel.value;
    if (n === '__new') n = await ask({ title: 'Nuova cartella', input: true, ok: 'Crea' });
    if (n) { f.folder = n; saveNow(); render(); }
    renderSheet();
  };
  const del = el('button', 'ic del'); del.setAttribute('aria-label', 'Rimuovi feed'); del.append(icon('trash'));
  let armed;
  del.onclick = () => {
    if (!armed) {
      armed = setTimeout(() => { armed = null; del.classList.remove('arm'); del.replaceChildren(icon('trash')); }, 3000);
      del.classList.add('arm'); del.textContent = 'Elimina?';
      return;
    }
    clearTimeout(armed);
    store.feeds = store.feeds.filter((x) => x !== f);
    store.items = store.items.filter((i) => i.feed !== f.url);
    pruneRead(); saveNow(); render(); renderSheet();
    toast('Feed rimosso');
  };
  r.append(st, tx, sel, del);
  return r;
}

async function doExport() {
  const feeds = store.feeds.filter((f) => !f.local).map((f) => ({ url: f.url, title: f.title, folder: f.folder }));
  if (!feeds.length) { toast('Non c’è nulla da esportare'); return; }
  let xml;
  try { xml = await backend.exportOpml(feeds); } catch (e) { toast(String(e)); return; }
  const box = $('#exportBox');
  box.hidden = false; box.value = xml; box.focus(); box.select();
  try { await navigator.clipboard.writeText(xml); toast('OPML copiato negli appunti'); }
  catch (_) { toast('Seleziona il testo e copialo'); }
}

/* ---------------------------------------------------------------- avvio */
function wire() {
  setIcon($('#bBack'), 'back'); setIcon($('#bRef'), 'refresh'); setIcon($('#bMenu'), 'menu'); setIcon($('#bClose'), 'close');
  $('#bBack').onclick = back;
  $('#bRef').onclick = () => refreshAll();
  $('#bMenu').onclick = () => go({ ...view, sheet: true, keepScroll: true }, true);
  $('#bClose').onclick = () => { if (history.state && history.state.sheet) history.back(); else go({ ...view, sheet: false, keepScroll: true }, false); };
  $('#addForm').onsubmit = (e) => { e.preventDefault(); addByInput(); };
  $('#inFile').onchange = (e) => { const f = e.target.files[0]; e.target.value = ''; if (f) importFile(f); };
  document.querySelector('label[for=inFile]').addEventListener('click', (e) => { e.preventDefault(); $('#inFile').click(); });
  $('#bExport').onclick = doExport;
  $('#segUnread').onclick = () => { store.settings.unreadOnly = true; save(); render(); };
  $('#segAll').onclick = () => { store.settings.unreadOnly = false; save(); render(); };
  $('#bMark').onclick = () => {
    const l = itemsOf(view.folder, view.feed);
    l.forEach((i) => { store.read[keyOf(i)] = 1; i.fresh = 0; });
    saveNow(); render(); toast('Segnati come letti');
  };
  $('#bMarkAll').onclick = async () => {
    if (!(await ask({ title: 'Segnare tutto come letto?', ok: 'Segna' }))) return;
    store.items.forEach((i) => { store.read[keyOf(i)] = 1; i.fresh = 0; });
    saveNow(); render(); toast('Tutto segnato come letto');
  };
  $('#bPurge').onclick = () => {
    const n = store.items.length;
    store.items = store.items.filter((i) => !isRead(i));
    pruneRead(); saveNow(); render(); renderSheet();
    toast(`Eliminati ${n - store.items.length} articoli`);
  };
  $('#setAuto').onchange = (e) => { store.settings.auto = +e.target.value; saveNow(); schedule(); };
  $('#setImages').onchange = (e) => { store.settings.images = e.target.checked; saveNow(); render(); };
  $('#setNotify').onchange = async (e) => {
    const on = e.target.checked;
    if (on && T && T.notification) {
      try {
        let ok = await T.notification.isPermissionGranted();
        if (!ok) ok = (await T.notification.requestPermission()) === 'granted';
        if (!ok) { e.target.checked = false; toast('Permesso notifiche negato'); return; }
      } catch (_) { e.target.checked = false; toast('Notifiche non disponibili'); return; }
    } else if (on && !native) {
      e.target.checked = false; toast('Le notifiche funzionano solo nell’app'); return;
    }
    store.settings.notify = on; saveNow(); renderSheet();
  };
}

let timer;
function schedule() {
  clearInterval(timer);
  const m = store.settings.auto;
  if (m > 0) timer = setInterval(() => { if (!document.hidden) refreshAll({ silent: true }); }, m * 60000);
}

load();
wire();
history.replaceState(view, '');
render();
schedule();
document.addEventListener('visibilitychange', () => {
  if (!document.hidden && store.settings.auto > 0 && store.feeds.length && Date.now() - store.lastRef > store.settings.auto * 60000) refreshAll({ silent: true });
});
if (store.feeds.length) refreshAll({ silent: true });
else go({ sheet: true }, false);
