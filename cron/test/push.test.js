import assert from 'node:assert/strict';
import { test } from 'node:test';

import { checkSubscription, cleanKeys, mergeSubscription, readIndex, recordId, validEndpoint, vapidAuthorization } from '../src/push.js';

test('only https endpoints of known push services are accepted', () => {
  assert.ok(validEndpoint('https://fcm.googleapis.com/fcm/send/abc'));
  assert.ok(validEndpoint('https://updates.push.services.mozilla.com/wpush/v2/abc'));
  assert.ok(validEndpoint('https://web.push.apple.com/abc'));
  assert.ok(validEndpoint('https://wns2-bl2p.notify.windows.com/w/?token=abc'));
  assert.equal(validEndpoint('http://fcm.googleapis.com/fcm/send/abc'), null);
  assert.equal(validEndpoint('https://evil.example/fcm.googleapis.com'), null);
  assert.equal(validEndpoint('https://fcm.googleapis.com.evil.example/x'), null);
  assert.equal(validEndpoint(42), null);
});

test('saved keys are filtered, deduplicated and capped', () => {
  assert.deepEqual(cleanKeys(['coto:prod1', 'coto:prod1', 'carrefour:42', 'jumbo:1', 'coto:', '<script>', 7]), ['coto:prod1', 'carrefour:42']);
  assert.equal(cleanKeys(Array.from({ length: 500 }, (_, i) => `dar:${i}`)).length, 300);
  assert.deepEqual(cleanKeys('coto:1'), []);
});

test('saving something already on offer never counts as coming back', () => {
  const first = mergeSubscription(null, 'e', ['coto:1', 'coto:2'], ['coto:1']);
  assert.deepEqual(first.onOffer, ['coto:1']);
  // Later sync: the server's state wins for known keys; a new key is seeded from the app.
  const later = mergeSubscription({ ...first, onOffer: ['coto:2'] }, 'e', ['coto:1', 'coto:2', 'dar:9'], ['coto:1', 'dar:9']);
  assert.deepEqual(later.onOffer, ['coto:2', 'dar:9']);
  // Removing a key drops its state.
  assert.deepEqual(mergeSubscription(later, 'e', ['coto:1'], []).onOffer, []);
});

test('a saved offer that comes back is reported once', () => {
  const record = { saved: ['coto:1', 'coto:2'], onOffer: ['coto:2'] };
  const today = checkSubscription(record, new Set(['coto:1', 'coto:2']), new Set(['coto']));
  assert.deepEqual(today.returned, ['coto:1']);
  assert.deepEqual(today.next, ['coto:1', 'coto:2']);
  const tomorrow = checkSubscription({ ...record, onOffer: today.next }, new Set(['coto:1', 'coto:2']), new Set(['coto']));
  assert.deepEqual(tomorrow.returned, []);
  assert.equal(tomorrow.changed, false);
});

test('a chain without data today keeps its state instead of looking gone', () => {
  const record = { saved: ['coto:1', 'dar:5'], onOffer: ['coto:1', 'dar:5'] };
  // DAR failed today: it is missing from the index.
  const failedDay = checkSubscription(record, new Set(['coto:1']), new Set(['coto']));
  assert.deepEqual(failedDay.next, ['coto:1', 'dar:5']);
  const nextDay = checkSubscription({ ...record, onOffer: failedDay.next }, new Set(['coto:1', 'dar:5']), new Set(['coto', 'dar']));
  assert.deepEqual(nextDay.returned, []);
});

test('reads keys and chains from the search index rows', () => {
  const { onOffer, chains } = readIndex([['coto:1', 'Coto', 'Yerba'], ['dar:2', 'DAR', 'Fideos'], 'basura']);
  assert.deepEqual([...onOffer], ['coto:1', 'dar:2']);
  assert.deepEqual([...chains], ['coto', 'dar']);
});

test('the VAPID header carries a valid ES256 token for the push service origin', async () => {
  const pair = await crypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, true, ['sign', 'verify']);
  const jwk = await crypto.subtle.exportKey('jwk', pair.privateKey);
  const header = await vapidAuthorization('https://fcm.googleapis.com/fcm/send/abc', jwk, 'PUBLICA', 'https://example.org', 0);
  const [, token, k] = /^vapid t=(.+), k=(.+)$/.exec(header);
  assert.equal(k, 'PUBLICA');
  const [head, body, signature] = token.split('.');
  const decode = (part) => JSON.parse(Buffer.from(part, 'base64url').toString());
  assert.deepEqual(decode(head), { typ: 'JWT', alg: 'ES256' });
  assert.deepEqual(decode(body), { aud: 'https://fcm.googleapis.com', exp: 12 * 3600, sub: 'https://example.org' });
  const valid = await crypto.subtle.verify(
    { name: 'ECDSA', hash: 'SHA-256' },
    pair.publicKey,
    Buffer.from(signature, 'base64url'),
    new TextEncoder().encode(`${head}.${body}`)
  );
  assert.ok(valid);
});

test('subscriptions are stored under a hash, never the endpoint itself', async () => {
  const id = await recordId('https://fcm.googleapis.com/fcm/send/secret');
  assert.match(id, /^sub:[A-Za-z0-9_-]{43}$/);
  assert.ok(!id.includes('secret'));
});
