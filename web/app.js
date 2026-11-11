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

// Header button: Chrome's install prompt when available, otherwise how to do it by hand.
// Hidden once the site runs as an installed app.
function setupInstall() {
  const button = $('#instalar-boton');
  const help = $('#instalar-ayuda');
  if (!button || !help) return;
  if (isInstalledApp()) {
    warmUpIndex();
    return;
  }
  button.hidden = false;

  let deferredPrompt = null;
  window.addEventListener('beforeinstallprompt', (event) => {
    event.preventDefault();
    deferredPrompt = event;
  });
  button.addEventListener('click', async () => {
    if (deferredPrompt) {
      deferredPrompt.prompt();
      await deferredPrompt.userChoice;
      deferredPrompt = null;
      return;
    }
    const isIos = /iphone|ipad|ipod/i.test(navigator.userAgent);
    help.textContent = isIos
      ? 'En iPhone: tocá Compartir (el cuadrado con la flecha) y después "Agregar a inicio". Queda como una app y la podés usar sin señal en el súper.'
      : 'En el menú del navegador (⋮) tocá "Instalar app" o "Agregar a pantalla principal". Queda como una app y la podés usar sin señal en el súper.';
    help.hidden = !help.hidden;
    button.setAttribute('aria-expanded', String(!help.hidden));
  });
  window.addEventListener('appinstalled', () => {
    button.hidden = true;
    help.hidden = true;
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

// ---------- Light / dark ----------
// theme.js applies the saved choice before paint. Without one, the site follows the device; the
// button flips whatever is showing now and remembers it on this device.

const THEME_KEY = 'ofertas-rosario:tema';
const darkQuery = window.matchMedia('(prefers-color-scheme: dark)');

function currentTheme() {
  return document.documentElement.dataset.theme || (darkQuery.matches ? 'dark' : 'light');
}

function setupTheme() {
  const button = $('#tema-boton');
  if (!button) return;
  const sync = () => {
    const dark = currentTheme() === 'dark';
    button.setAttribute('aria-pressed', String(dark));
    $('#tema-texto').textContent = dark ? 'Cambiar a modo claro' : 'Cambiar a modo oscuro';
    button.title = $('#tema-texto').textContent;
  };
  button.hidden = false;
  sync();
  button.addEventListener('click', () => {
    const next = currentTheme() === 'dark' ? 'light' : 'dark';
    document.documentElement.dataset.theme = next;
    try {
      localStorage.setItem(THEME_KEY, next);
    } catch {
      // Storage blocked: the choice lasts until the page is closed.
    }
    sync();
  });
  darkQuery.addEventListener('change', sync);
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
    button.className = 'btn';
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

// Small DOM helper: element with a class and children (strings become text, never HTML).
function el(tag, className, ...children) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  node.append(...children.filter((child) => child !== null && child !== undefined && child !== ''));
  return node;
}

// Same icons as templates/icons.html (decorative: hidden from screen readers).
const ICONS = {
  external: 'M7 17 17 7M9 7h8v8',
};

function icon(name) {
  const ns = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(ns, 'svg');
  const attrs = { class: 'icon', viewBox: '0 0 24 24', fill: 'none', stroke: 'currentColor', 'stroke-width': '2.2', 'stroke-linecap': 'round', 'stroke-linejoin': 'round', 'aria-hidden': 'true', focusable: 'false' };
  for (const [key, value] of Object.entries(attrs)) svg.setAttribute(key, value);
  const path = document.createElementNS(ns, 'path');
  path.setAttribute('d', ICONS[name]);
  svg.append(path);
  return svg;
}

// Promo text worth showing next to the badge (same rule as promo_chip in src/site.rs): a bare
// "35%Dto" repeats the badge and DAR's "Oferta" is on every offer, so neither is shown.
function promoChip(promo) {
  const text = (promo || '').trim();
  if (!text || /^oferta$/i.test(text) || /^\d+\s*%\s*dto\.?$/i.test(text)) return '';
  return text;
}

// Same markup as templates/offers.html, so search results and saved offers look identical.
function renderList(rows) {
  const list = el('ol', 'cards');
  for (const [key, chainName, name, price, listPrice, discount, promo, url, image] of rows) {
    const item = el('li', 'card');
    item.dataset.id = key;
    if (image) {
      // Thumbnail from the chain's image server (same attributes as the server-rendered pages).
      const img = document.createElement('img');
      img.src = image;
      img.alt = '';
      img.width = 80;
      img.height = 80;
      img.loading = 'lazy';
      img.decoding = 'async';
      img.referrerPolicy = 'no-referrer';
      item.append(img);
    }
    const priceLine = el('p', 'card-price', el('strong', 'price', formatArs(price)));
    if (listPrice) priceLine.append(' ', el('span', 'before', 'antes ', el('s', '', formatArs(listPrice))));
    let tags = null;
    const chip = promoChip(promo);
    if (listPrice || chip) {
      tags = el('p', 'card-tags');
      if (listPrice) tags.append(el('em', discount >= 40 ? 'badge badge-big' : 'badge', '-' + discount + '%'));
      if (chip) tags.append(' ', el('span', 'chip', chip));
    }
    const actions = el('p', 'card-actions');
    if (url) {
      const link = el('a', 'card-link', 'ver en ' + chainName + ' ', icon('external'));
      link.href = url;
      link.rel = 'nofollow noopener';
      link.target = '_blank';
      actions.append(link);
    }
    actions.append(saveButton(key));
    item.append(el('div', 'card-body', el('b', 'card-name', name), el('p', 'card-chain', chainName), priceLine, tags, actions));
    list.append(item);
  }
  return list;
}

// ---------- Search and filters ----------
// Everything runs over /datos/buscar.json (all of today's offers). The filters live in the URL
// (?q=&cadena=&min=&promo=&desde=&hasta=&orden=) so a filtered view can be shared or restored with
// "back". On a chain page the form has a single hidden chain box: searches stay inside that chain
// and, with no filter, the page shows its own server-rendered list again.

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

// Chain page: one fixed (hidden) chain box instead of the chain chips.
function isChainPage() {
  return chainBoxes().length === 1;
}

// ---------- Validation ----------
// Prices are whole pesos, typed as "1500", "1.500" or "$ 1.500" (dots as thousands separators, the
// way prices are written here). Anything else is an error shown under the fields, and while there is
// one the price filter is not applied and stays out of the URL.

const MAX_PESOS = 10000000;
const MAX_QUERY = 80;
const PESOS_FORMAT = /^(\d{1,3}(\.\d{3})+|\d+)$/;

function parsePesos(raw) {
  const text = raw.trim().replace(/^\$\s*/, '');
  if (!text) return { value: 0 };
  if (!PESOS_FORMAT.test(text)) return { error: 'Usá solo números enteros, por ejemplo 1500 o 1.500.' };
  const value = Number(text.replace(/\./g, ''));
  if (value > MAX_PESOS) return { error: 'El precio máximo es $10.000.000.' };
  return { value };
}

function readPrice() {
  const from = parsePesos($('#f-desde').value);
  const to = parsePesos($('#f-hasta').value);
  const invalid = { from: Boolean(from.error), to: Boolean(to.error) };
  let error = from.error || to.error || '';
  if (!error && from.value && to.value && from.value > to.value) {
    error = '"Desde" tiene que ser menor o igual que "Hasta".';
    invalid.from = invalid.to = true;
  }
  return { from: error ? 0 : from.value, to: error ? 0 : to.value, error, invalid };
}

// Marks the price fields and shows the message (an empty live region when everything is fine).
function showPriceValidation(price) {
  $('#f-desde').setAttribute('aria-invalid', String(price.invalid.from));
  $('#f-hasta').setAttribute('aria-invalid', String(price.invalid.to));
  const message = $('#precio-error');
  if (message.textContent !== price.error) message.textContent = price.error;
}

// Words of one letter are ignored by the search: say so instead of silently showing nothing new.
function showQueryHint(filters) {
  const hint = $('#q-ayuda');
  const text = filters.q.trim() && !searchWords(filters.q).length ? 'Escribí al menos 2 letras para buscar.' : '';
  if (hint.textContent !== text) hint.textContent = text;
}

function searchWords(q) {
  return normalize(q).split(/\s+/).filter((word) => word.length > 1);
}

// On leaving a price field, write the number the way it is read: "1500" becomes "1.500".
function formatPesosField(input) {
  const parsed = parsePesos(input.value);
  if (!parsed.error) input.value = parsed.value ? parsed.value.toLocaleString('es-AR') : '';
}

function readFilters() {
  const chains = chainBoxes().filter((box) => box.checked).map((box) => box.value);
  const price = readPrice();
  return {
    q: $('#q').value,
    chains,
    allChains: chains.length === chainBoxes().length,
    min: Number($('#f-min').value) || 0,
    promo: $('#f-promo').value,
    from: price.from,
    to: price.to,
    price,
    orden: $('#f-orden').value,
  };
}

function filtersFromUrl() {
  const params = new URLSearchParams(location.search);
  $('#q').value = (params.get('q') || '').slice(0, MAX_QUERY);
  // Unknown chains in the URL are ignored; if none is known, every chain stays checked.
  const known = new Set(chainBoxes().map((box) => box.value));
  const wanted = (params.get('cadena') || '').split(',').filter((id) => known.has(id));
  chainBoxes().forEach((box) => (box.checked = !wanted.length || wanted.includes(box.value)));
  const setSelect = (id, value) => {
    const select = $(id);
    if ([...select.options].some((option) => option.value === value)) select.value = value;
  };
  setSelect('#f-min', params.get('min') || '');
  setSelect('#f-promo', params.get('promo') || '');
  setSelect('#f-orden', params.get('orden') || '');
  const setPesos = (id, value) => {
    const parsed = parsePesos(/^\d+$/.test(value || '') ? value : '');
    $(id).value = parsed.value ? parsed.value.toLocaleString('es-AR') : '';
  };
  setPesos('#f-desde', params.get('desde'));
  setPesos('#f-hasta', params.get('hasta'));
}

function filtersToUrl(filters) {
  const params = new URLSearchParams();
  if (filters.q.trim()) params.set('q', filters.q.trim());
  if (!filters.allChains) params.set('cadena', filters.chains.join(','));
  if (filters.min) params.set('min', String(filters.min));
  if (filters.promo) params.set('promo', filters.promo);
  if (filters.from) params.set('desde', String(filters.from));
  if (filters.to) params.set('hasta', String(filters.to));
  if (filters.orden) params.set('orden', filters.orden);
  const query = params.toString().replace(/%2C/gi, ',');
  history.replaceState(null, '', location.pathname + (query ? '?' + query : '') + location.hash);
}

let emptyState = null;

function setupSearch() {
  const form = $('#filtros');
  const results = $('#resultados');
  if (!form || !results) return;
  // What the server rendered in the results area: the shortcuts on the home page, the chain's own
  // list (and pager) on a chain page. Shown again, as the same node, when no filter is active.
  emptyState = results.firstElementChild;
  form.querySelectorAll('[disabled]').forEach((control) => (control.disabled = false));
  filtersFromUrl();

  let timer = null;
  const update = () => {
    shown = PAGE_SIZE;
    const filters = readFilters();
    showPriceValidation(filters.price);
    showQueryHint(filters);
    filtersToUrl(filters);
    renderResults(filters, results);
  };
  form.addEventListener('submit', (event) => {
    event.preventDefault();
    update();
  });
  form.addEventListener('change', update);
  form.addEventListener('change', updateFilterCount);
  updateFilterCount();
  // "Limpiar filtros": back to every chain, any discount, promo and price, default order. Keeps the text.
  $('#limpiar-filtros').addEventListener('click', () => {
    chainBoxes().forEach((box) => (box.checked = true));
    ['#f-min', '#f-promo', '#f-desde', '#f-hasta', '#f-orden'].forEach((id) => ($(id).value = ''));
    updateFilterCount();
    update();
    $('#q').focus();
  });
  // The filters panel is a <details>: always open on wide screens (its toggle is hidden there).
  const panel = form.querySelector('.filters-panel');
  const wide = window.matchMedia('(min-width: 1024px)');
  const syncPanel = () => {
    if (panel && wide.matches) panel.open = true;
  };
  syncPanel();
  wide.addEventListener('change', syncPanel);
  // Shortcuts in the empty state ("50% o más", "2x1 y 3x2", "yerba"...): apply them in place
  // instead of reloading the page. Delegated, because the empty state is taken out and put back.
  results.addEventListener('click', (event) => {
    const link = event.target.closest('a[data-quick]');
    if (!link || event.ctrlKey || event.metaKey || event.shiftKey || event.button !== 0) return;
    event.preventDefault();
    history.replaceState(null, '', link.pathname + link.search + location.hash);
    filtersFromUrl();
    updateFilterCount();
    update();
    if (results.getBoundingClientRect().top < 0) results.scrollIntoView({ block: 'start' });
  });
  $('#q').addEventListener('focus', () => loadIndex().catch(() => {}), { once: true });
  ['#f-desde', '#f-hasta'].forEach((id) => $(id).addEventListener('change', (event) => formatPesosField(event.target)));
  // Text and prices apply while typing, after a short pause.
  ['#q', '#f-desde', '#f-hasta'].forEach((id) =>
    $(id).addEventListener('input', () => {
      clearTimeout(timer);
      timer = setTimeout(() => {
        update();
        updateFilterCount();
      }, 300);
    })
  );
  if (location.search) {
    const filters = readFilters();
    showPriceValidation(filters.price);
    showQueryHint(filters);
    renderResults(filters, results);
  }
}

// "Filtros (2)": how many filters differ from the defaults, shown on the closed panel's toggle.
function updateFilterCount() {
  const counter = $('#filtros-cuenta');
  if (!counter) return;
  const filters = readFilters();
  const active = [!filters.allChains, filters.min, filters.promo, filters.from || filters.to, filters.orden].filter(Boolean).length;
  counter.textContent = active ? '(' + active + ')' : '';
  const clear = $('#limpiar-filtros');
  if (clear) clear.hidden = active === 0;
}

const SORTS = {
  '': (a, b) => (b[5] || 0) - (a[5] || 0) || a[3] - b[3],
  precio: (a, b) => a[3] - b[3],
  nombre: (a, b) => a[2].localeCompare(b[2], 'es'),
};

async function renderResults(filters, results) {
  const words = searchWords(filters.q);
  // On a chain page, a different order alone also needs the full list (the page only has 100).
  const filtering = words.length || !filters.allChains || filters.min || filters.promo || filters.from || filters.to || (isChainPage() && filters.orden);
  if (!filtering) {
    // Back to what the server rendered (shortcuts, or the chain's list). Its save buttons may be
    // stale: they were detached while results were shown.
    if (emptyState) {
      results.replaceChildren(emptyState);
      emptyState.querySelectorAll('button[data-fav]').forEach((button) => syncButton(button, isSaved(button.dataset.fav)));
    } else {
      results.replaceChildren();
    }
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
    .filter((row) => !filters.from || row[3] >= filters.from * 100)
    .filter((row) => !filters.to || row[3] <= filters.to * 100)
    .sort(SORTS[filters.orden] || SORTS['']);

  const parts = [];
  const summary = document.createElement('p');
  summary.className = 'results-count';
  summary.textContent = matches.length
    ? matches.length + (matches.length === 1 ? ' oferta' : ' ofertas') + (matches.length > shown ? '. Mostrando ' + shown + '.' : '.')
    : 'No hay ofertas con estos filtros hoy.';
  parts.push(summary);
  if ((filters.min || filters.promo === 'rebaja') && chains.has('dar')) {
    const note = document.createElement('p');
    note.className = 'results-hint';
    note.textContent = 'DAR no publica precio anterior: no aparece al filtrar por descuento.';
    parts.push(note);
  }
  // How to see the same price on each chain's site, for the chains in the shown results (a chain
  // page already says it above the form).
  const shownChains = new Set(isChainPage() ? [] : matches.slice(0, shown).map((row) => row[0].split(':')[0]));
  for (const box of chainBoxes()) {
    if (shownChains.has(box.value) && box.dataset.hint) {
      const hint = document.createElement('p');
      hint.className = 'results-hint';
      hint.textContent = box.dataset.name + ': ' + box.dataset.hint.charAt(0).toLowerCase() + box.dataset.hint.slice(1);
      parts.push(hint);
    }
  }
  if (matches.length) parts.push(renderList(matches.slice(0, shown)));
  if (matches.length > shown) {
    const more = document.createElement('button');
    more.type = 'button';
    more.className = 'btn btn-primary';
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
  button.className = 'btn-save';
  button.dataset.fav = key;
  syncButton(button, isSaved(key));
  return button;
}

function syncButton(button, saved) {
  button.hidden = false;
  button.dataset.saved = String(saved);
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
    note.className = 'lead';
    const list = el('ul', 'gone-list');
    for (const item of gone) {
      list.append(el('li', '', el('span', '', (item.name || 'Un producto guardado') + ' (' + chainOf(item.key) + ')'), saveButton(item.key)));
    }
    container.append(note, list);
  }
}

// ---------- Back to top ----------
// Floating button for long lists, shown only after scrolling down a couple of screens.

function setupToTop() {
  const button = $('#arriba');
  if (!button) return;
  let ticking = false;
  const sync = () => {
    button.hidden = window.scrollY < window.innerHeight * 1.5;
    ticking = false;
  };
  window.addEventListener(
    'scroll',
    () => {
      if (!ticking) {
        ticking = true;
        requestAnimationFrame(sync);
      }
    },
    { passive: true }
  );
  button.addEventListener('click', () => {
    const smooth = !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    window.scrollTo({ top: 0, behavior: smooth ? 'smooth' : 'auto' });
    // Keyboard users land at the top too, not on a button that is about to hide.
    const brand = document.querySelector('.brand');
    if (brand) brand.focus({ preventScroll: true });
  });
  sync();
}

document.addEventListener('DOMContentLoaded', () => {
  setupTheme();
  setupToTop();
  setupInstall();
  setupFreshness();
  setupSearch();
  setupSaved();
});
