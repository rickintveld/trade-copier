---
name: add-tauri-command
description: Checklist for adding or changing a Tauri IPC command end-to-end (Rust command, handler registration, typed frontend wrapper, tests). Use whenever a frontend feature needs new data or an action from the backend, or when a command's arguments/return shape changes.
---

# Add or change a Tauri command

The IPC boundary spans three files that must stay in sync. A mismatch does not fail to compile — it fails silently at runtime.

## 1. Rust command — `backend/src/tauri_commands.rs`

```rust
#[tauri::command]
pub async fn get_thing(
    state: State<'_, AppState>,
    worker_id: Option<i64>,          // JS sends `workerId`
) -> Result<ApiResponse<serde_json::Value>, String> {
    match state.db.get_thing(worker_id).await {
        Ok(v) => Ok(ApiResponse { success: true, data: serde_json::to_value(v).map_err(|e| e.to_string())? }),
        Err(e) => Err(e.to_string()),
    }
}
```

- Return `Result<ApiResponse<T>, String>`; errors become a rejected promise in JS.
- Never spawn or stop workers directly — send `WorkerCommand::{Start, Stop}` via `state.worker_command_tx`.
- DB access goes through a `Database` method (see the `add-db-column` skill for schema changes).

## 2. Register — `backend/src/main.rs`

Add `tauri_commands::get_thing` to the `tauri::generate_handler![...]` list. Forgetting this gives `command get_thing not found` at runtime only.

## 3. Frontend wrapper — `frontend/lib/api.ts`

- Add/extend an `Api*` interface mirroring the Rust struct **field names in snake_case** (serde output).
- Add a method to `tradeCopierApi` using `tauriInvoke<T>('get_thing', { workerId })`.
- **Argument keys must be camelCase.** Tauri 2 converts camelCase JS keys to snake_case Rust params by default; snake_case keys are silently dropped (the param arrives as `None`). `frontend/lib/api.test.ts` guards this.
- If the UI needs it, add a camelCase type in `frontend/types/trading.ts` and a transform in `frontend/lib/transforms.ts`; parse timestamps with `parseUTCTimestamp` from `frontend/lib/time.ts`.

## 4. Tests

- Rust: test the `Database` method in `database.rs`'s `#[cfg(test)]` module (`Database::new(":memory:")`).
- Frontend: extend `frontend/lib/api.test.ts` (asserts invoke name + args) and `transforms.test.ts`.

## 5. Docs and verify

- Update `docs/api.md`.
- `cargo clippy --all-targets -- -D warnings && cargo test`
- `npm run typecheck && npm run lint && npm test`
