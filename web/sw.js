// Service worker: keeps today's offers available offline (no signal inside the supermarket).
// __VERSION__ and __PRECACHE__ are replaced by the build, so every deploy gets a fresh cache;
// __PUSH_URL__ by `push_url` in config/site.toml (the alerts Worker).
'use strict';

const CACHE = 'ofertas-rosario-__VERSION__';
const PRECACHE = __PRECACHE__;
const NETWORK_TIMEOUT_MS = 4000;

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(CACHE)
      .then((cache) => cache.addAll(PRECACHE))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((key) => key !== CACHE).map((key) => caches.delete(key))))
      .then(() => self.clients.claim()),
  );
});

// Network first (prices change every day), cache as fallback when offline or slow.
self.addEventListener('fetch', (event) => {
  const request = event.request;
  const url = new URL(request.url);
  if (request.method !== 'GET' || url.origin !== self.location.origin) return;
  // Freshness check (see app.js): always straight to the network, never from the cache.
  if (url.pathname === '/datos/actualizado.json') return;
  event.respondWith(networkFirst(request));
});

async function networkFirst(request) {
  const cache = await caches.open(CACHE);
  try {
    const response = await withTimeout(fetch(request), NETWORK_TIMEOUT_MS);
    if (response.ok) cache.put(request, response.clone());
    return response;
  } catch {
    const cached = await cache.match(request, { ignoreSearch: true });
    if (cached) return cached;
    if (request.mode === 'navigate') {
      const home = await cache.match('/');
      if (home) return home;
    }
    return new Response('Sin conexión', { status: 503, headers: { 'Content-Type': 'text/plain; charset=utf-8' } });
  }
}

// ---------- "Volvió a estar en oferta" alerts ----------
// Pushes carry no data: the Worker in cron/ sends one only when a saved offer came back, and this
// asks it which ones and names them from today's offers. Browsers require every push to show a
// notification, so any failure falls back to a generic text.
const PUSH_URL = '__PUSH_URL__';

self.addEventListener('push', (event) => event.waitUntil(showReturned()));

async function showReturned() {
  let names = [];
  try {
    const subscription = await self.registration.pushManager.getSubscription();
    const response = await fetch(PUSH_URL + '/novedades', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ endpoint: subscription.endpoint }),
    });
    const { returned } = await response.json();
    const rows = await (await fetch('/datos/buscar.json')).json();
    const byKey = new Map(rows.map((row) => [row[0], row[2]]));
    names = returned.map((key) => byKey.get(key)).filter(Boolean);
  } catch {
    // Generic notification below.
  }
  const title = names.length > 1 ? 'Volvieron a estar en oferta' : 'Volvió a estar en oferta';
  const shown = names.slice(0, 3).join(', ');
  const body = !names.length
    ? 'Algunos de tus guardados están en oferta hoy.'
    : names.length > 3
      ? shown + ' y ' + (names.length - 3) + ' más.'
      : shown + '.';
  return self.registration.showNotification(title, {
    body,
    icon: '/icons/icon-192.png',
    tag: 'guardados',
    data: { url: '/#guardados' },
  });
}

self.addEventListener('notificationclick', (event) => {
  event.notification.close();
  const url = new URL(event.notification.data.url, self.location.origin).href;
  event.waitUntil(
    self.clients.matchAll({ type: 'window', includeUncontrolled: true }).then((windows) => {
      const open = windows.find((client) => new URL(client.url).origin === self.location.origin);
      if (open) return open.navigate(url).then((client) => (client || open).focus());
      return self.clients.openWindow(url);
    }),
  );
});

function withTimeout(promise, ms) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('timeout')), ms);
    promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (error) => {
        clearTimeout(timer);
        reject(error);
      },
    );
  });
}
