---
name: add-db-column
description: How to add a table, column or seeded default to the SQLite schema in backend/src/database.rs without breaking existing user databases. Use for any schema change.
---

# Change the SQLite schema

There is no migration framework. `Database::new()` runs every time the app starts against the user's existing DB at `dirs::data_local_dir()/trade-copier/trade_copier.db`, so every statement must be safe to re-run.

## New table

Add a `CREATE TABLE IF NOT EXISTS ...` (and `CREATE INDEX IF NOT EXISTS ...`) inside the `conn.call` block in `Database::new()`.

## New column on an existing table

1. Add the column to the `CREATE TABLE IF NOT EXISTS` statement (for fresh installs).
2. Add an idempotent migration for existing installs, ignoring the "duplicate column" error on purpose:

   ```rust
   let _ = conn.execute(
       "ALTER TABLE workers ADD COLUMN my_column TEXT NOT NULL DEFAULT ''",
       [],
   );
   ```

   SQLite `ADD COLUMN` needs a constant default for `NOT NULL` columns. Never drop or rename columns — older app versions and existing data depend on them.

## Seeded defaults

Use `INSERT OR IGNORE` (see the `feature_toggles` seeds) so user changes survive restarts.

## Then

- Update the `*Record` struct and every `SELECT` that maps rows by index (`row.get(n)`) — column order matters.
- Timestamps: use `CURRENT_TIMESTAMP` (UTC, no zone). The frontend must parse them with `parseUTCTimestamp`.
- If the field is exposed over IPC, follow the `add-tauri-command` skill for the `Api*` interface.
- Add a test in the `#[cfg(test)] mod tests` of `database.rs`, including `schema_creation_is_idempotent_and_seeds_feature_toggles` still passing (it opens the same file twice).
- Update `docs/database.md`.
