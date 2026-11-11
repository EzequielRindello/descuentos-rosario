// Browser side: install prompt, offline service worker, search and saved offers.
// No framework and no build step on purpose.
'use strict';

const SAVED_KEY = 'ofertas-rosario:guardados';

const $ = (selector) => document.querySelector(selector);

// ---------- Service worker (offline) ----------

if ('serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js').catch((error) => console.warn('Service worker not registered', error));
  });
}

// ---------- Install ----------

function setupInstall() {
  const section = $('#instalar');
  if (!section) return;
  if (isInstalledApp()) {
    warmUpIndex();
    return;
  }

  const isIos = /iphone|ipad|ipod/i.test(navigator.userAgent);
  section.hidden = false;
  if (isIos) {
    $('#instalar-ios').hidden = false;
  } else {
    $('#instalar-android').hidden = false;
  }

  let deferredPrompt = null;
  const button = $('#instalar-boton');
  window.addEventListener('beforeinstallprompt', (event) => {
    event.preventDefault();
    deferredPrompt = event;
    button.hidden = false;
    $('#instalar-android').hidden = true;
  });
  button.addEventListener('click', async () => {
    if (!deferredPrompt) return;
    deferredPrompt.prompt();
    await deferredPrompt.userChoice;
    deferredPrompt = null;
    button.hidden = true;
  });
  window.addEventListener('appinstalled', () => {
    section.hidden = true;
    warmUpIndex();
  });
}

// The installed app is the one used offline in the supermarket: download the search index in the
// background so the service worker caches it. Browser visits only download it when searching.
function warmUpIndex() {
  const run = () => loadIndex().catch(() => {});
  if ('requestIdleCallback' in window) requestIdleCallback(run, { timeout: 5000 });
  else setTimeout(run, 2000);
}

function isInstalledApp() {
  return window.matchMedia('(display-mode: standalone)').matches || window.navigator.standalone === true;
}

// ---------- Freshness notice ----------
// Pages carry the time of their data (body[data-updated]). The service worker may serve an old
// copy (offline, or slow network), so compare it with /datos/actualizado.json, which the service
// worker never caches, and say so when the offers on screen are not today's.

const TIME_ZONE = 'America/Argentina/Buenos_Aires';
const dayKey = (date) => new Intl.DateTimeFormat('en-CA', { timeZone: TIME_ZONE }).format(date);
const dayLabel = (date) => new Intl.DateTimeFormat('es-AR', { timeZone: TIME_ZONE, day: '2-digit', month: '2-digit' }).format(date);

async function fetchLatestUpdate() {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 5000);
  try {
    const response = await fetch('/datos/actualizado.json', { cache: 'no-store', signal: controller.signal });
    if (!response.ok) return { state: 'unknown' };
    const date = new Date((await response.json()).generated_at);
    return Number.isNaN(date.getTime()) ? { state: 'unknown' } : { state: 'online', date };
  } catch {
    return { state: 'offline' };
  } finally {
    clearTimeout(timer);
  }
}

async function checkFreshness() {
  const notice = $('#aviso-fecha');
  const pageDate = new Date(document.body.dataset.updated || '');
  if (!notice || Number.isNaN(pageDate.getTime())) return;
  const fromToday = dayKey(pageDate) === dayKey(new Date());
  const latest = await fetchLatestUpdate();

  notice.replaceChildren();
  if (latest.state === 'online' && latest.date > pageDate) {
    const button = document.createElement('button');
    button.type = 'button';
    button.textContent = 'Recargar';
    button.addEventListener('click', () => location.reload());
    notice.append('Hay ofertas más nuevas (del ' + dayLabel(latest.date) + '). ', button);
  } else if (fromToday) {
    notice.hidden = true;
    return;
  } else if (latest.state === 'offline') {
    notice.textContent = 'Sin conexión: estas ofertas son del ' + dayLabel(pageDate) + ' y pueden haber cambiado.';
  } else if (latest.state === 'online') {
    notice.textContent = 'Todavía no están las ofertas de hoy (se actualizan a la mañana). Estas son del ' + dayLabel(pageDate) + '.';
  } else {
    notice.textContent = 'Estas ofertas son del ' + dayLabel(pageDate) + ' y pueden haber cambiado.';
  }
  notice.hidden = false;
}

function setupFreshness() {
  checkFreshness();
  // An installed app can stay open for days: check again when it comes back or reconnects.
  window.addEventListener('online', checkFreshness);
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') checkFreshness();
  });
}

// ---------- Data ----------

// Rows of /datos/buscar.json: [key, chainName, name, price, listPrice, discountPct, promo, url].
let indexPromise = null;
function loadIndex() {
  if (!indexPromise) {
    indexPromise = fetch('/datos/buscar.json')
      .then((response) => {
        if (!response.ok) throw new Error('HTTP ' + response.status);
        return response.json();
      })
      .then((rows) => rows.map((row) => ({ row, text: normalize(row[2] + ' ' + row[1]) })));
  }
  return indexPromise;
}

function normalize(text) {
  return text.toLowerCase().normalize('NFD').replace(/[̀-ͯ]/g, '');
}

function formatArs(cents) {
  const pesos = Math.floor(cents / 100);
  const rest = cents % 100;
  const integer = pesos.toLocaleString('es-AR');
  return '$' + integer + (rest ? ',' + String(rest).padStart(2, '0') : '');
}

function renderList(rows) {
  const list = document.createElement('ol');
  list.className = 'ofertas';
  for (const [key, chainName, name, price, listPrice, discount, promo, url] of rows) {
    const item = document.createElement('li');
    item.dataset.id = key;
    const parts = [];
    const title = document.createElement('b');
    title.textContent = name;
    item.append(title, ' · ' + chainName + ' · ');
    const strong = document.createElement('strong');
    strong.textContent = formatArs(price);
    item.append(strong);
    if (listPrice) {
      item.append(' · antes ');
      const before = document.createElement('s');
      before.textContent = formatArs(listPrice);
      const off = document.createElement('em');
      off.textContent = '-' + discount + '%';
      item.append(before, ' · ', off);
    }
    if (promo) parts.push(promo);
    if (parts.length) item.append(' · ' + parts.join(' · '));
    if (url) {
      item.append(' · ');
      const link = document.createElement('a');
      link.href = url;
      link.rel = 'nofollow noopener';
      link.target = '_blank';
      link.textContent = 'ver en ' + chainName;
      item.append(link);
    }
    item.append(' ');
    item.append(saveButton(key));
    list.append(item);
  }
  return list;
}

// ---------- Search and filters ----------
// Everything runs over /datos/buscar.json (all of today's offers). The filters live in the URL
// (?q=&cadena=&min=&promo=&orden=) so a filtered view can be shared or restored with "back".

const PAGE_SIZE = 50;
let shown = PAGE_SIZE;

// Kind of promotion, from the chain's promo text and the discount.
function promoKind(row) {
  const promo = row[6] || '';
  if (/\d\s*x\s*\d/i.test(promo)) return 'nxm';
  if (/2da/i.test(promo)) return '2da';
  if (/llevando/i.test(promo)) return 'varias';
  return row[5] ? 'rebaja' : '';
}

function chainBoxes() {
  return [...document.querySelectorAll('#filtros input[name="cadena"]')];
}

function readFilters() {
  const chains = chainBoxes().filter((box) => box.checked).map((box) => box.value);
  return {
    q: $('#q').value,
    chains,
    allChains: chains.length === chainBoxes().length,
    min: Number($('#f-min').value) || 0,
    promo: $('#f-promo').value,
    orden: $('#f-orden').value,
  };
}

function filtersFromUrl() {
  const params = new URLSearchParams(location.search);
  $('#q').value = params.get('q') || '';
  const wanted = (params.get('cadena') || '').split(',').filter(Boolean);
  if (wanted.length) chainBoxes().forEach((box) => (box.checked = wanted.includes(box.value)));
  const setSelect = (id, value) => {
    const select = $(id);
    if ([...select.options].some((option) => option.value === value)) select.value = value;
  };
  setSelect('#f-min', params.get('min') || '');
  setSelect('#f-promo', params.get('promo') || '');
  setSelect('#f-orden', params.get('orden') || '');
}

function filtersToUrl(filters) {
  const params = new URLSearchParams();
  if (filters.q.trim()) params.set('q', filters.q.trim());
  if (!filters.allChains) params.set('cadena', filters.chains.join(','));
  if (filters.min) params.set('min', String(filters.min));
  if (filters.promo) params.set('promo', filters.promo);
  if (filters.orden) params.set('orden', filters.orden);
  const query = params.toString().replace(/%2C/gi, ',');
  history.replaceState(null, '', location.pathname + (query ? '?' + query : '') + location.hash);
}

function setupSearch() {
  const form = $('#filtros');
  const results = $('#resultados');
  if (!form || !results) return;
  form.querySelectorAll('[disabled]').forEach((control) => (control.disabled = false));
  filtersFromUrl();

  let timer = null;
  const update = () => {
    shown = PAGE_SIZE;
    const filters = readFilters();
    filtersToUrl(filters);
    renderResults(filters, results);
  };
  form.addEventListener('submit', (event) => {
    event.preventDefault();
    update();
  });
  form.addEventListener('change', update);
  $('#q').addEventListener('focus', () => loadIndex().catch(() => {}), { once: true });
  $('#q').addEventListener('input', () => {
    clearTimeout(timer);
    timer = setTimeout(update, 200);
  });
  if (location.search) renderResults(readFilters(), results);
}

const SORTS = {
  '': (a, b) => (b[5] || 0) - (a[5] || 0) || a[3] - b[3],
  precio: (a, b) => a[3] - b[3],
  nombre: (a, b) => a[2].localeCompare(b[2], 'es'),
};

async function renderResults(filters, results) {
  const words = normalize(filters.q).split(/\s+/).filter((word) => word.length > 1);
  const filtering = words.length || !filters.allChains || filters.min || filters.promo;
  if (!filtering) {
    results.replaceChildren();
    return;
  }
  let index;
  try {
    index = await loadIndex();
  } catch {
    results.textContent = 'No se pudieron cargar las ofertas. Probá de nuevo en un rato.';
    return;
  }
  const chains = new Set(filters.chains);
  const matches = index
    .filter((entry) => words.every((word) => entry.text.includes(word)))
    .map((entry) => entry.row)
    .filter((row) => chains.has(row[0].split(':')[0]))
    .filter((row) => !filters.min || (row[5] || 0) >= filters.min)
    .filter((row) => !filters.promo || promoKind(row) === filters.promo)
    .sort(SORTS[filters.orden] || SORTS['']);

  const parts = [];
  const summary = document.createElement('p');
  summary.textContent = matches.length
    ? matches.length + (matches.length === 1 ? ' oferta' : ' ofertas') + (matches.length > shown ? '. Mostrando ' + shown + '.' : '.')
    : 'No hay ofertas con estos filtros hoy.';
  parts.push(summary);
  if ((filters.min || filters.promo === 'rebaja') && chains.has('dar')) {
    const note = document.createElement('p');
    note.textContent = 'DAR no publica precio anterior: no aparece al filtrar por descuento.';
    parts.push(note);
  }
  if (matches.length) parts.push(renderList(matches.slice(0, shown)));
  if (matches.length > shown) {
    const more = document.createElement('button');
    more.type = 'button';
    more.textContent = 'Mostrar ' + Math.min(PAGE_SIZE, matches.length - shown) + ' más';
    more.addEventListener('click', () => {
      shown += PAGE_SIZE;
      renderResults(filters, results);
    });
    parts.push(more);
  }
  results.replaceChildren(...parts);
}

// ---------- Saved offers (only on this device) ----------

// Stored as [{ key, name }]. The name is kept so an offer that is gone today can still be listed
// (and discarded). Older versions stored plain keys; those are read with no name.
function readSaved() {
  try {
    const items = JSON.parse(localStorage.getItem(SAVED_KEY) || '[]');
    return items.map((item) => (typeof item === 'string' ? { key: item, name: null } : item)).filter((item) => item && item.key);
  } catch {
    return [];
  }
}

function writeSaved(items) {
  try {
    localStorage.setItem(SAVED_KEY, JSON.stringify(items));
  } catch {
    // Private mode or storage blocked: saving just doesn't persist.
  }
}

function isSaved(key) {
  return readSaved().some((item) => item.key === key);
}

function saveButton(key) {
  const button = document.createElement('button');
  button.type = 'button';
  button.dataset.fav = key;
  syncButton(button, isSaved(key));
  return button;
}

function syncButton(button, saved) {
  button.hidden = false;
  button.textContent = saved ? 'Descartar' : 'Guardar';
}

const CHAIN_NAMES = { carrefour: 'Carrefour', coto: 'Coto', dar: 'DAR' };

function chainOf(key) {
  const id = key.split(':')[0];
  return CHAIN_NAMES[id] || id;
}

function setupSaved() {
  document.querySelectorAll('button[data-fav]').forEach((button) => syncButton(button, isSaved(button.dataset.fav)));

  document.addEventListener('click', (event) => {
    const button = event.target.closest('button[data-fav]');
    if (!button) return;
    const key = button.dataset.fav;
    const items = readSaved();
    const saved = items.some((item) => item.key === key);
    if (saved) {
      writeSaved(items.filter((item) => item.key !== key));
    } else {
      const title = button.closest('li') && button.closest('li').querySelector('b');
      writeSaved([...items, { key, name: title ? title.textContent : null }]);
    }
    document.querySelectorAll('button[data-fav="' + CSS.escape(key) + '"]').forEach((b) => syncButton(b, !saved));
    renderSaved();
  });
  renderSaved();
}

async function renderSaved() {
  const section = $('#guardados');
  const container = $('#guardados-lista');
  if (!section || !container) return;
  const items = readSaved();
  section.hidden = items.length === 0;
  if (!items.length) return;
  let index;
  try {
    index = await loadIndex();
  } catch {
    container.textContent = 'No se pudieron cargar tus guardados.';
    return;
  }
  const byKey = new Map(index.map((entry) => [entry.row[0], entry.row]));
  // Fill in names for items saved by older versions, while their offer is still listed.
  if (items.some((item) => !item.name && byKey.has(item.key))) {
    writeSaved(items.map((item) => (!item.name && byKey.has(item.key) ? { ...item, name: byKey.get(item.key)[2] } : item)));
  }
  const rows = items.map((item) => byKey.get(item.key)).filter(Boolean);
  const gone = items.filter((item) => !byKey.has(item.key));
  container.replaceChildren();
  if (rows.length) container.append(renderList(rows));
  if (gone.length) {
    const note = document.createElement('p');
    note.textContent = gone.length === 1 ? 'Hoy no está en oferta (si vuelve, aparece de nuevo arriba):' : 'Hoy no están en oferta (si vuelven, aparecen de nuevo arriba):';
    const list = document.createElement('ul');
    for (const item of gone) {
      const entry = document.createElement('li');
      entry.append((item.name || 'Un producto guardado') + ' (' + chainOf(item.key) + ') ', saveButton(item.key));
      list.append(entry);
    }
    container.append(note, list);
  }
}

document.addEventListener('DOMContentLoaded', () => {
  setupInstall();
  setupFreshness();
  setupSearch();
  setupSaved();
});
