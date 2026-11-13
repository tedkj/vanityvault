-- Everything the backup server keeps. No column holds a password, a private key or an address.

CREATE TABLE accounts (
  id TEXT PRIMARY KEY,                 -- chosen by the app, random
  username TEXT NOT NULL UNIQUE,
  salt TEXT NOT NULL,                  -- for the password step, which runs on the user's PC
  kdf TEXT NOT NULL,                   -- the password step's settings
  auth_hash TEXT NOT NULL,             -- SHA-256 of the sign-in proof
  wrapped_key TEXT NOT NULL,           -- the vault key, locked by the password
  recovery_hash TEXT NOT NULL,         -- SHA-256 of the recovery proof
  wrapped_key_recovery TEXT NOT NULL,  -- the vault key, locked by the recovery key
  created INTEGER NOT NULL
);

CREATE TABLE entries (
  account_id TEXT NOT NULL,
  id TEXT NOT NULL,                    -- chosen by the app, says nothing about the key
  nonce TEXT NOT NULL,
  ct TEXT NOT NULL,                    -- the locked box
  size INTEGER NOT NULL,
  created INTEGER NOT NULL,
  updated INTEGER NOT NULL,
  PRIMARY KEY (account_id, id)
) WITHOUT ROWID;

CREATE TABLE sessions (
  token_hash TEXT PRIMARY KEY,
  account_id TEXT NOT NULL,
  expires INTEGER NOT NULL
);
CREATE INDEX sessions_account ON sessions (account_id);

-- Wrong sign-in tries per username, for names with and without an account.
CREATE TABLE throttle (
  username TEXT PRIMARY KEY,
  fails INTEGER NOT NULL,
  locked_until INTEGER NOT NULL,
  updated INTEGER NOT NULL
);

CREATE TABLE meta (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
