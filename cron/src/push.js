// Pure logic for the "volvió a estar en oferta" notifications, kept apart from the Worker entry
// so it can be tested with `node --test cron/` (Node has the same fetch and WebCrypto APIs).

/** Push services browsers use. Anything else is refused, so the Worker never posts to random hosts. */
const PUSH_HOSTS = [/^fcm\.googleapis\.com$/, /^updates\.push\.services\.mozilla\.com$/, /^web\.push\.apple\.com$/, /\.notify\.windows\.com$/];

/** A saved offer key, as app.js writes it: "<chain>:<product id>". */
const KEY = /^(carrefour|coto|dar):[A-Za-z0-9._-]{1,64}$/;

/** More saved products than anyone keeps by hand; the cap bounds the KV value size. */
export const MAX_SAVED = 300;

export function chainOf(key) {
  return key.split(':')[0];
}

/** Returns the endpoint when it is an https URL of a known push service, else null. */
export function validEndpoint(value) {
  if (typeof value !== 'string' || value.length > 1000) return null;
  let url;
  try {
    url = new URL(value);
  } catch {
    return null;
  }
  if (url.protocol !== 'https:' || !PUSH_HOSTS.some((host) => host.test(url.hostname))) return null;
  return url.href;
}

/** Keeps only well-formed, unique keys, at most MAX_SAVED. */
export function cleanKeys(value) {
  if (!Array.isArray(value)) return [];
  return [...new Set(value.filter((key) => typeof key === 'string' && KEY.test(key)))].slice(0, MAX_SAVED);
}

/**
 * Stored record after the app sends its saved list. `clientOnOffer` is what the app sees on offer
 * today: it seeds only keys that are new to the server, so saving something already on offer never
 * counts as "it came back". Keys the server already tracks keep the server's own state.
 */
export function mergeSubscription(previous, endpoint, saved, clientOnOffer) {
  const known = new Set(previous ? previous.saved : []);
  const wanted = new Set(saved);
  const kept = (previous ? previous.onOffer : []).filter((key) => wanted.has(key));
  const seeded = clientOnOffer.filter((key) => wanted.has(key) && !known.has(key));
  return {
    endpoint,
    saved,
    onOffer: [...new Set([...kept, ...seeded])].sort(),
    lastReturned: previous ? previous.lastReturned || [] : [],
  };
}

/**
 * What came back for one subscription: saved keys on offer today that were not on offer last time.
 * Chains missing from today's data (a failed scrape publishes them empty) keep yesterday's state,
 * so their products don't look gone today and "come back" tomorrow.
 */
export function checkSubscription(record, onOffer, chainsWithData) {
  const previous = new Set(record.onOffer);
  const nowOn = record.saved.filter((key) => onOffer.has(key));
  const returned = nowOn.filter((key) => !previous.has(key));
  const unknownToday = record.onOffer.filter((key) => record.saved.includes(key) && !chainsWithData.has(chainOf(key)));
  const next = [...new Set([...nowOn, ...unknownToday])].sort();
  const changed = next.join() !== [...record.onOffer].sort().join();
  return { returned, next, changed };
}

/** Keys on offer today and the chains that have data, from the rows of /datos/buscar.json. */
export function readIndex(rows) {
  const onOffer = new Set();
  const chains = new Set();
  for (const row of Array.isArray(rows) ? rows : []) {
    if (!Array.isArray(row) || typeof row[0] !== 'string') continue;
    onOffer.add(row[0]);
    chains.add(chainOf(row[0]));
  }
  return { onOffer, chains };
}

// ---------- VAPID (RFC 8292) ----------

export function base64url(bytes) {
  let text = '';
  for (const byte of new Uint8Array(bytes)) text += String.fromCharCode(byte);
  return btoa(text).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}

function base64urlText(text) {
  return base64url(new TextEncoder().encode(text));
}

/**
 * `Authorization` header for a push to `endpoint`. `privateJwk` is the VAPID private key as a JWK
 * (P-256), `publicKey` the matching raw key in base64url (the same one the page subscribes with).
 */
export async function vapidAuthorization(endpoint, privateJwk, publicKey, subject, now = Date.now()) {
  const key = await crypto.subtle.importKey('jwk', privateJwk, { name: 'ECDSA', namedCurve: 'P-256' }, false, ['sign']);
  const header = base64urlText(JSON.stringify({ typ: 'JWT', alg: 'ES256' }));
  const claims = base64urlText(JSON.stringify({ aud: new URL(endpoint).origin, exp: Math.floor(now / 1000) + 12 * 3600, sub: subject }));
  const unsigned = `${header}.${claims}`;
  // WebCrypto's ECDSA signature is already the raw r||s form that JWT ES256 expects.
  const signature = await crypto.subtle.sign({ name: 'ECDSA', hash: 'SHA-256' }, key, new TextEncoder().encode(unsigned));
  return `vapid t=${unsigned}.${base64url(signature)}, k=${publicKey}`;
}

/** KV key for a subscription: the endpoint itself is long and works like a password, so it is hashed. */
export async function recordId(endpoint) {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(endpoint));
  return 'sub:' + base64url(digest);
}
