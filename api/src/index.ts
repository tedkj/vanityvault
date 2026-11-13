// The VanityVault backup server.
//
// It stores locked boxes and nothing that opens them. Everything is encrypted by the app on the
// user's PC before it gets here, so this code never sees a password, a private key or an address.
//
// What the server holds for an account:
// - the username, the password salt and the password-step settings (needed to sign in);
// - SHA-256 of `auth_key`, which proves who is signing in and decrypts nothing;
// - the account's vault key, wrapped twice by the app (once by the password, once by the recovery key);
// - SHA-256 of `recovery_auth`, which proves the user holds the recovery key;
// - entries: an id chosen by the app, a nonce and ciphertext.
//
// Nothing here writes a request body, a key or a token to the logs.

interface Statement {
  bind(...values: unknown[]): Statement;
  first<T = Record<string, unknown>>(): Promise<T | null>;
  all<T = Record<string, unknown>>(): Promise<{ results: T[] }>;
  run(): Promise<{ meta: { changes: number } }>;
}
interface Env {
  DB: { prepare(sql: string): Statement; batch(statements: Statement[]): Promise<unknown[]> };
  RL_AUTH?: { limit(o: { key: string }): Promise<{ success: boolean }> };
  RL_API?: { limit(o: { key: string }): Promise<{ success: boolean }> };
}

const MAX_BODY = 600_000;
const MAX_ENTRIES = 20_000;
const MAX_PER_CALL = 200;
const PAGE = 1000;
const SESSION_SECONDS = 2 * 3600;
const DEFAULT_KDF = { m_kib: 262_144, t: 3, p: 1 };

class Fail extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message);
  }
}

const json = (status: number, body: unknown) =>
  new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json', 'cache-control': 'no-store' } });
const now = () => Math.floor(Date.now() / 1000);
const enc = new TextEncoder();

function b64(bytes: Uint8Array): string {
  let s = '';
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s);
}
function hex(bytes: ArrayBuffer): string {
  return [...new Uint8Array(bytes)].map((b) => b.toString(16).padStart(2, '0')).join('');
}
async function sha256(text: string): Promise<string> {
  return hex(await crypto.subtle.digest('SHA-256', enc.encode(text)));
}
/** Compares without stopping at the first difference. */
function same(a: string, b: string): boolean {
  let diff = a.length ^ b.length;
  for (let i = 0; i < a.length; i++) diff |= a.charCodeAt(i) ^ b.charCodeAt(i % (b.length || 1));
  return diff === 0;
}

// --- checks on what the app sends

type Body = Record<string, unknown>;

/** Base64 text of exactly `bytes` bytes, or of at most `max` characters when `bytes` is 0. */
function base64(v: unknown, name: string, bytes: number, max = 0): string {
  const ok = typeof v === 'string' && /^[A-Za-z0-9+/]+={0,2}$/.test(v) && v.length % 4 === 0;
  if (!ok || (bytes ? (v as string).length !== Math.ceil(bytes / 3) * 4 : (v as string).length > max)) throw new Fail(400, `Bad ${name}.`);
  return v as string;
}
function username(v: unknown): string {
  const u = typeof v === 'string' ? v.trim().toLowerCase() : '';
  // A made-up name, never an email address: we don't want to know who anyone is.
  if (u.includes('@')) throw new Fail(400, "Make up a username instead of using your email address. We don't want to know it.");
  if (!/^[a-z0-9][a-z0-9._-]{5,31}$/.test(u)) throw new Fail(400, 'Usernames have 6 to 32 letters, digits, dots, dashes or underscores.');
  return u;
}
function kdf(v: unknown): string {
  const k = (v ?? {}) as Record<string, unknown>;
  const int = (x: unknown, lo: number, hi: number) => Number.isInteger(x) && (x as number) >= lo && (x as number) <= hi;
  if (!int(k.m_kib, 8, 1_048_576) || !int(k.t, 1, 10) || !int(k.p, 1, 4)) throw new Fail(400, 'Bad password settings.');
  return JSON.stringify({ m_kib: k.m_kib, t: k.t, p: k.p });
}
/** A wrapped vault key: a 24-byte nonce and 48 bytes of ciphertext. */
function wrapped(v: unknown, name: string): string {
  const w = (v ?? {}) as Record<string, unknown>;
  return JSON.stringify({ nonce: base64(w.nonce, name, 24), ct: base64(w.ct, name, 48) });
}

// --- throttling

/** Per address: a cap on how fast one sender can call. */
async function limit(req: Request, limiter: Env['RL_API'], what: string) {
  const ip = req.headers.get('cf-connecting-ip') ?? 'local';
  if (limiter && !(await limiter.limit({ key: `${what}:${ip}` })).success) throw new Fail(429, 'Too many requests. Wait a minute and try again.');
}

// Per username: wrong tries are counted whether or not the account exists, so the counting
// can't be used to find out which usernames are taken. From the fifth wrong try the wait doubles.
async function gate(env: Env, user: string) {
  const row = await env.DB.prepare('SELECT locked_until FROM throttle WHERE username = ?').bind(user).first<{ locked_until: number }>();
  const wait = (row?.locked_until ?? 0) - now();
  if (wait > 0) throw new Fail(429, `Too many wrong tries. Wait ${wait} seconds and try again.`);
}
async function strike(env: Env, user: string) {
  const t = now();
  await env.DB.prepare(
    `INSERT INTO throttle (username, fails, locked_until, updated) VALUES (?1, 1, 0, ?2)
     ON CONFLICT (username) DO UPDATE SET
       locked_until = CASE WHEN fails + 1 >= 5 THEN ?2 + MIN(900, 30 * (1 << MIN(fails - 4, 5))) ELSE 0 END,
       fails = fails + 1, updated = ?2`,
  )
    .bind(user, t)
    .run();
  if (Math.random() < 0.02) await env.DB.prepare('DELETE FROM throttle WHERE updated < ?').bind(t - 86_400).run();
}
const pardon = (env: Env, user: string) => env.DB.prepare('DELETE FROM throttle WHERE username = ?').bind(user).run();

// --- sessions

async function open(env: Env, account: string): Promise<string> {
  const token = b64(crypto.getRandomValues(new Uint8Array(32)));
  const t = now();
  await env.DB.batch([
    env.DB.prepare('DELETE FROM sessions WHERE expires < ?').bind(t),
    env.DB.prepare('INSERT INTO sessions (token_hash, account_id, expires) VALUES (?, ?, ?)').bind(await sha256(token), account, t + SESSION_SECONDS),
  ]);
  return token;
}
function bearer(req: Request): string {
  const h = req.headers.get('authorization') ?? '';
  if (!h.startsWith('Bearer ') || h.length > 100) throw new Fail(401, 'You are signed out. Sign in again.');
  return h.slice(7);
}
async function account(req: Request, env: Env): Promise<string> {
  const row = await env.DB.prepare('SELECT account_id FROM sessions WHERE token_hash = ? AND expires > ?')
    .bind(await sha256(bearer(req)), now())
    .first<{ account_id: string }>();
  if (!row) throw new Fail(401, 'You are signed out. Sign in again.');
  return row.account_id;
}

// --- the secret behind made-up salts

let saltKey: Promise<CryptoKey> | undefined;
function getSaltKey(env: Env): Promise<CryptoKey> {
  return (saltKey ??= (async () => {
    const fresh = b64(crypto.getRandomValues(new Uint8Array(32)));
    await env.DB.prepare("INSERT OR IGNORE INTO meta (key, value) VALUES ('salt_key', ?)").bind(fresh).run();
    const row = await env.DB.prepare("SELECT value FROM meta WHERE key = 'salt_key'").first<{ value: string }>();
    return crypto.subtle.importKey('raw', enc.encode(row!.value), { name: 'HMAC', hash: 'SHA-256' }, false, ['sign']);
  })().catch((e) => {
    saltKey = undefined;
    throw e;
  }));
}

// --- the calls

type Account = { id: string; salt: string; kdf: string; auth_hash: string; wrapped_key: string; recovery_hash: string; wrapped_key_recovery: string };
const find = (env: Env, user: string) => env.DB.prepare('SELECT * FROM accounts WHERE username = ?').bind(user).first<Account>();

const calls: Record<string, (b: Body, env: Env, req: Request) => Promise<unknown>> = {
  // The salt and settings the app needs to turn a password into keys. An unknown username gets a
  // made-up salt that is the same every time, so this call doesn't tell who has an account.
  async prelogin(b, env) {
    const user = username(b.username);
    const row = await find(env, user);
    if (row) return { salt: row.salt, kdf: JSON.parse(row.kdf) };
    const mac = await crypto.subtle.sign('HMAC', await getSaltKey(env), enc.encode(user));
    return { salt: b64(new Uint8Array(mac).slice(0, 16)), kdf: DEFAULT_KDF };
  },

  async register(b, env) {
    const user = username(b.username);
    const id = typeof b.account_id === 'string' && /^[0-9a-f]{32}$/.test(b.account_id) ? b.account_id : '';
    if (!id) throw new Fail(400, 'Bad account id.');
    const values = [
      id,
      user,
      base64(b.salt, 'salt', 16),
      kdf(b.kdf),
      await sha256(base64(b.auth_key, 'key', 32)),
      wrapped(b.wrapped_key, 'wrapped key'),
      await sha256(base64(b.recovery_auth, 'recovery key', 32)),
      wrapped(b.wrapped_key_recovery, 'wrapped key'),
      now(),
    ];
    const done = await env.DB.prepare(
      `INSERT OR IGNORE INTO accounts (id, username, salt, kdf, auth_hash, wrapped_key, recovery_hash, wrapped_key_recovery, created)
       VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
    )
      .bind(...values)
      .run();
    if (!done.meta.changes) throw new Fail(409, 'That username is taken.');
    return { token: await open(env, id) };
  },

  async login(b, env) {
    const user = username(b.username);
    const hash = await sha256(base64(b.auth_key, 'key', 32));
    await gate(env, user);
    const row = await find(env, user);
    if (!row || !same(hash, row.auth_hash)) {
      await strike(env, user);
      throw new Fail(401, 'Wrong username or password.');
    }
    await pardon(env, user);
    return { token: await open(env, row.id), account_id: row.id, wrapped_key: JSON.parse(row.wrapped_key) };
  },

  async logout(_b, env, req) {
    await env.DB.prepare('DELETE FROM sessions WHERE token_hash = ?').bind(await sha256(bearer(req))).run();
    return {};
  },

  // Forgotten password, step 1: prove the recovery key and get the vault key it wraps.
  async 'recover/start'(b, env) {
    const row = await byRecovery(b, env);
    return { account_id: row.id, wrapped_key_recovery: JSON.parse(row.wrapped_key_recovery) };
  },
  // Step 2: set the new password. Everyone signed in to the account is signed out.
  async 'recover/finish'(b, env) {
    const row = await byRecovery(b, env);
    await setPassword(env, row.id, b);
    await pardon(env, username(b.username));
    return { token: await open(env, row.id) };
  },

  async password(b, env, req) {
    const id = await account(req, env);
    await proven(env, id, b.auth_key);
    await setPassword(env, id, { ...b, auth_key: b.new_auth_key });
    return { token: await open(env, id) };
  },

  async 'account/delete'(b, env, req) {
    const id = await account(req, env);
    await proven(env, id, b.auth_key);
    await env.DB.batch([
      env.DB.prepare('DELETE FROM entries WHERE account_id = ?').bind(id),
      env.DB.prepare('DELETE FROM sessions WHERE account_id = ?').bind(id),
      env.DB.prepare('DELETE FROM accounts WHERE id = ?').bind(id),
    ]);
    return {};
  },

  async 'entries/list'(b, env, req) {
    const id = await account(req, env);
    const after = typeof b.after === 'string' ? b.after : '';
    const { results } = await env.DB.prepare('SELECT id, nonce, ct, updated FROM entries WHERE account_id = ? AND id > ? ORDER BY id LIMIT ?')
      .bind(id, after, PAGE + 1)
      .all();
    return { entries: results.slice(0, PAGE), more: results.length > PAGE };
  },

  async 'entries/put'(b, env, req) {
    const id = await account(req, env);
    const list = Array.isArray(b.entries) ? (b.entries as Body[]) : [];
    if (!list.length || list.length > MAX_PER_CALL) throw new Fail(400, `Send 1 to ${MAX_PER_CALL} entries at a time.`);
    const rows = list.map((e) => ({ id: entryId(e.id), nonce: base64(e.nonce, 'entry', 24), ct: base64(e.ct, 'entry', 0, 1024) }));
    const ids = JSON.stringify(rows.map((r) => r.id));
    const count = await env.DB.prepare(
      `SELECT (SELECT COUNT(*) FROM entries WHERE account_id = ?1) AS total,
              (SELECT COUNT(*) FROM entries WHERE account_id = ?1 AND id IN (SELECT value FROM json_each(?2))) AS known`,
    )
      .bind(id, ids)
      .first<{ total: number; known: number }>();
    if (count!.total - count!.known + new Set(rows.map((r) => r.id)).size > MAX_ENTRIES) {
      throw new Fail(413, `A backup can hold ${MAX_ENTRIES} keys, and this one is full.`);
    }
    await env.DB.prepare(
      `INSERT INTO entries (account_id, id, nonce, ct, size, created, updated)
       SELECT ?1, value ->> 'id', value ->> 'nonce', value ->> 'ct', length(value ->> 'ct'), ?2, ?2 FROM json_each(?3) WHERE true
       ON CONFLICT (account_id, id) DO UPDATE SET nonce = excluded.nonce, ct = excluded.ct, size = excluded.size, updated = excluded.updated`,
    )
      .bind(id, now(), JSON.stringify(rows))
      .run();
    return { stored: rows.length };
  },

  async 'entries/delete'(b, env, req) {
    const id = await account(req, env);
    const ids = Array.isArray(b.ids) ? b.ids.map(entryId) : [];
    if (!ids.length || ids.length > MAX_PER_CALL) throw new Fail(400, `Send 1 to ${MAX_PER_CALL} ids at a time.`);
    const done = await env.DB.prepare('DELETE FROM entries WHERE account_id = ? AND id IN (SELECT value FROM json_each(?))')
      .bind(id, JSON.stringify(ids))
      .run();
    return { deleted: done.meta.changes };
  },
};

function entryId(v: unknown): string {
  if (typeof v !== 'string' || !/^[0-9a-f]{16}$/.test(v)) throw new Fail(400, 'Bad entry id.');
  return v;
}

/** The account, if the caller proves they hold its recovery key. */
async function byRecovery(b: Body, env: Env): Promise<Account> {
  const user = username(b.username);
  const hash = await sha256(base64(b.recovery_auth, 'recovery key', 32));
  await gate(env, user);
  const row = await find(env, user);
  if (!row || !same(hash, row.recovery_hash)) {
    await strike(env, user);
    throw new Fail(401, 'That username and recovery key do not belong together.');
  }
  return row;
}

/** Stops unless `authKey` is the account's current one. */
async function proven(env: Env, id: string, authKey: unknown) {
  const hash = await sha256(base64(authKey, 'key', 32));
  const row = await env.DB.prepare('SELECT auth_hash FROM accounts WHERE id = ?').bind(id).first<{ auth_hash: string }>();
  if (!row || !same(hash, row.auth_hash)) throw new Fail(403, 'Wrong password.');
}

/** New salt, new sign-in proof and the vault key wrapped again, all in one step. Ends every session. */
async function setPassword(env: Env, id: string, b: Body) {
  const values = [base64(b.salt, 'salt', 16), kdf(b.kdf), await sha256(base64(b.auth_key, 'key', 32)), wrapped(b.wrapped_key, 'wrapped key'), id];
  await env.DB.batch([
    env.DB.prepare('UPDATE accounts SET salt = ?, kdf = ?, auth_hash = ?, wrapped_key = ? WHERE id = ?').bind(...values),
    env.DB.prepare('DELETE FROM sessions WHERE account_id = ?').bind(id),
  ]);
}

const SIGN_IN = new Set(['prelogin', 'register', 'login', 'recover/start', 'recover/finish']);

async function handle(req: Request, env: Env): Promise<Response> {
  const path = new URL(req.url).pathname;
  if (req.method === 'GET' && path === '/v1/health') return json(200, { ok: true });
  const call = path.startsWith('/v1/') ? path.slice(4) : '';
  if (req.method !== 'POST' || !Object.hasOwn(calls, call)) throw new Fail(404, 'Not found.');
  await limit(req, SIGN_IN.has(call) ? env.RL_AUTH : env.RL_API, SIGN_IN.has(call) ? 'auth' : 'api');
  if (Number(req.headers.get('content-length') ?? 0) > MAX_BODY) throw new Fail(413, 'Too much data in one request.');
  const text = await req.text();
  if (text.length > MAX_BODY) throw new Fail(413, 'Too much data in one request.');
  let body: unknown;
  try {
    body = JSON.parse(text);
  } catch {
    throw new Fail(400, 'Bad request.');
  }
  if (!body || typeof body !== 'object' || Array.isArray(body)) throw new Fail(400, 'Bad request.');
  return json(200, await calls[call](body as Body, env, req));
}

export default {
  async fetch(req: Request, env: Env): Promise<Response> {
    try {
      return await handle(req, env);
    } catch (e) {
      if (e instanceof Fail) return json(e.status, { error: e.message });
      return json(500, { error: 'The backup server had a problem. Try again in a moment.' });
    }
  },
};
