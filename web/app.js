// Browser side: install prompt, offline service worker, search and saved offers.
// No framework and no build step on purpose.
'use strict';

const SAVED_KEY = 'ofertas-rosario:guardados';
const MAX_RESULTS = 50;

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
  const standalone = window.matchMedia('(display-mode: standalone)').matches || window.navigator.standalone === true;
  if (standalone) return;

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

// ---------- Search ----------

function setupSearch() {
  const input = $('#q');
  const results = $('#resultados');
  if (!input || !results) return;
  input.disabled = false;

  let timer = null;
  input.addEventListener('focus', () => loadIndex().catch(() => {}), { once: true });
  input.addEventListener('input', () => {
    clearTimeout(timer);
    timer = setTimeout(() => search(input.value, results), 200);
  });
}

async function search(query, results) {
  const words = normalize(query).split(/\s+/).filter((word) => word.length > 1);
  if (!words.length) {
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
  const matches = index
    .filter((entry) => words.every((word) => entry.text.includes(word)))
    .map((entry) => entry.row)
    .sort((a, b) => (b[5] || 0) - (a[5] || 0) || a[3] - b[3]);

  const summary = document.createElement('p');
  summary.textContent = matches.length
    ? matches.length + (matches.length === 1 ? ' oferta' : ' ofertas') + (matches.length > MAX_RESULTS ? ', se muestran las ' + MAX_RESULTS + ' con más descuento' : '') + '.'
    : 'No hay ofertas para "' + query.trim() + '" hoy.';
  results.replaceChildren(summary);
  if (matches.length) results.append(renderList(matches.slice(0, MAX_RESULTS)));
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
  setupSearch();
  setupSaved();
});
