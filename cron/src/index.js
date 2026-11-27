// Worker apart from the site (which stays static files only). Two jobs:
// 1. 12:00 (Argentina): starts daily.yml on main through GitHub's workflow_dispatch API.
// 2. 12:30: "volvió a estar en oferta" push notifications for saved offers, after that publish.
// It also receives the subscriptions from the site (POST /suscripcion, /baja and /novedades).
// Each day is independent: a failed run is logged and the next day's trigger fires anyway.

import { checkSubscription, cleanKeys, mergeSubscription, readIndex, recordId, validEndpoint, vapidAuthorization } from './push.js';

const DISPATCH_URL = 'https://api.github.com/repos/EzequielRindello/descuentos-rosario/actions/workflows/daily.yml/dispatches';
const USER_AGENT = 'ofertas-rosario-cron (+https://github.com/EzequielRindello/descuentos-rosario)';
/** Must match the second cron in ../wrangler.jsonc. */
const NOTIFY_CRON = '30 15 * * *';
/** The free plan allows 50 outgoing requests per run: one is the offers index, the rest pushes. */
const MAX_PUSHES = 45;
const MAX_BODY = 32 * 1024;

export default {
  async scheduled(controller, env) {
    if (controller.cron === NOTIFY_CRON) await notify(env);
    else await dispatchDaily(env);
  },

  async fetch(request, env) {
    const origin = request.headers.get('Origin') || '';
    const allowed = env.SITE_URL === origin || (env.EXTRA_ORIGIN && env.EXTRA_ORIGIN === origin);
    const cors = allowed ? { 'Access-Control-Allow-Origin': origin, Vary: 'Origin' } : {};
    if (request.method === 'OPTIONS') {
      return new Response(null, {
        status: allowed ? 204 : 403,
        headers: { ...cors, 'Access-Control-Allow-Methods': 'POST', 'Access-Control-Allow-Headers': 'Content-Type', 'Access-Control-Max-Age': '86400' },
      });
    }
    if (!allowed) return new Response('Origen no permitido', { status: 403 });
    if (request.method !== 'POST') return new Response('Método no permitido', { status: 405, headers: cors });

    const text = await request.text();
    if (text.length > MAX_BODY) return new Response('Pedido demasiado grande', { status: 413, headers: cors });
    let body;
    try {
      body = JSON.parse(text);
    } catch {
      return new Response('JSON inválido', { status: 400, headers: cors });
    }
    const endpoint = validEndpoint(body && body.endpoint);
    if (!endpoint) return new Response('Suscripción inválida', { status: 400, headers: cors });
    const id = await recordId(endpoint);
    const path = new URL(request.url).pathname;

    if (path === '/suscripcion') {
      const saved = cleanKeys(body.saved);
      if (!saved.length) {
        await env.SUBSCRIPTIONS.delete(id);
        return new Response(null, { status: 204, headers: cors });
      }
      const previous = await env.SUBSCRIPTIONS.get(id, 'json');
      await env.SUBSCRIPTIONS.put(id, JSON.stringify(mergeSubscription(previous, endpoint, saved, cleanKeys(body.onOffer))));
      return new Response(null, { status: 204, headers: cors });
    }
    if (path === '/baja') {
      await env.SUBSCRIPTIONS.delete(id);
      return new Response(null, { status: 204, headers: cors });
    }
    if (path === '/novedades') {
      // The service worker asks what came back, to write the notification (pushes carry no data).
      const record = await env.SUBSCRIPTIONS.get(id, 'json');
      return Response.json({ returned: record ? record.lastReturned || [] : [] }, { headers: cors });
    }
    return new Response('No existe', { status: 404, headers: cors });
  },
};

async function dispatchDaily(env) {
  const response = await fetch(DISPATCH_URL, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${env.GITHUB_TOKEN}`,
      Accept: 'application/vnd.github+json',
      'X-GitHub-Api-Version': '2022-11-28',
      'User-Agent': USER_AGENT,
    },
    body: JSON.stringify({ ref: 'main' }),
  });
  // GitHub answers 204 on success. Anything else (401: expired or revoked token) marks this run as
  // failed in the Worker's logs.
  if (!response.ok) {
    throw new Error(`GitHub rejected the dispatch: ${response.status} ${await response.text()}`);
  }
}

async function notify(env) {
  if (!env.VAPID_PUBLIC_KEY || !env.VAPID_PRIVATE_KEY) {
    console.log('Notifications off: VAPID keys not configured');
    return;
  }
  const index = await fetch(`${env.SITE_URL}/datos/buscar.json`, { headers: { 'User-Agent': USER_AGENT } });
  if (!index.ok) throw new Error(`Could not read today's offers: ${index.status}`);
  const { onOffer, chains } = readIndex(await index.json());
  const privateJwk = JSON.parse(env.VAPID_PRIVATE_KEY);

  let sent = 0;
  let pending = 0;
  let removed = 0;
  let cursor;
  do {
    const page = await env.SUBSCRIPTIONS.list({ prefix: 'sub:', cursor });
    cursor = page.list_complete ? undefined : page.cursor;
    for (const { name } of page.keys) {
      const record = await env.SUBSCRIPTIONS.get(name, 'json');
      if (!record) continue;
      const { returned, next, changed } = checkSubscription(record, onOffer, chains);
      if (!returned.length) {
        if (changed) await env.SUBSCRIPTIONS.put(name, JSON.stringify({ ...record, onOffer: next }));
        continue;
      }
      // Over the cap: left as is, so it is sent on the next run.
      if (sent >= MAX_PUSHES) {
        pending++;
        continue;
      }
      const response = await fetch(record.endpoint, {
        method: 'POST',
        headers: {
          Authorization: await vapidAuthorization(record.endpoint, privateJwk, env.VAPID_PUBLIC_KEY, env.SITE_URL),
          TTL: String(12 * 3600),
          Urgency: 'normal',
          'Content-Length': '0',
        },
      });
      sent++;
      if (response.status === 404 || response.status === 410) {
        // The browser dropped the subscription (notifications turned off, app uninstalled).
        await env.SUBSCRIPTIONS.delete(name);
        removed++;
      } else if (response.ok) {
        await env.SUBSCRIPTIONS.put(name, JSON.stringify({ ...record, onOffer: next, lastReturned: returned }));
      } else {
        // Left unchanged: tried again on the next run.
        console.error(`Push rejected: ${response.status} ${await response.text()}`);
      }
    }
  } while (cursor);
  console.log(`Notifications: ${sent} sent, ${removed} subscriptions removed, ${pending} left for the next run`);
}
