-- When each account was last signed in to. Accounts unused for 24 months are deleted with
-- everything in them, by the daily cleanup in src/index.ts. Accounts that exist today start
-- their 24 months now.

ALTER TABLE accounts ADD COLUMN last_seen INTEGER NOT NULL DEFAULT 0;
UPDATE accounts SET last_seen = CAST(strftime('%s', 'now') AS INTEGER);
CREATE INDEX accounts_last_seen ON accounts (last_seen);
