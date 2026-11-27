// Generates the VAPID key pair for the push notifications. Run once (node cron/scripts/vapid-keys.mjs)
// and keep the output out of the repo:
// - "privada" goes as the Worker secret VAPID_PRIVATE_KEY (npx wrangler@4.155.0 secret put ...).
// - "publica" goes in cron/wrangler.jsonc (VAPID_PUBLIC_KEY) and config/site.toml (vapid_public_key).
// Changing the pair later invalidates every subscription: people have to turn the alerts on again.
import { webcrypto } from 'node:crypto';

const { publicKey, privateKey } = await webcrypto.subtle.generateKey({ name: 'ECDSA', namedCurve: 'P-256' }, true, ['sign', 'verify']);
const raw = Buffer.from(await webcrypto.subtle.exportKey('raw', publicKey)).toString('base64url');
const jwk = await webcrypto.subtle.exportKey('jwk', privateKey);
console.log('publica:', raw);
console.log('privada:', JSON.stringify({ kty: jwk.kty, crv: jwk.crv, x: jwk.x, y: jwk.y, d: jwk.d }));
