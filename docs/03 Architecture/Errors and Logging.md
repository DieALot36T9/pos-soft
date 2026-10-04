---
tags: [architecture, errors, logging]
---
# Errors and Logging

## Error model
```rust
// core::AppError: the only error type that crosses into the UI
pub enum AppError {
    Validation { field: Option<String>, code: String, params: Map },   // user can fix the input
    NotFound { entity: String, key: String },
    Conflict { code: String, params: Map },                              // e.g. barcode already used
    PermissionDenied { permission: Permission },
    OverrideRequired { permission: Permission },                         // UI shows the override dialog
    Hardware { device: String, code: String, detail: String },          // printer offline, scale timeout
    Internal { id: String },                                             // a bug: details only in the log
}
```
- `code` values are stable snake_case strings (`barcode_in_use`, `credit_limit_exceeded`). The UI looks up the plain-English message for each code in `shared/text/en/errors.ts`.
- Each module has its own error enum (`SalesError`) with `impl From<SalesError> for AppError`.
- `Internal` errors log the full chain with an id. The user sees "Something went wrong (ref ABC123)".
- **No `unwrap()`/`expect()`** outside tests and startup code (clippy `unwrap_used` denied in module crates).

## Logging
| Item | Decision |
|---|---|
| Library | `tracing` + `tracing-subscriber` + `tracing-appender` |
| Always on | WARN and ERROR to the log file |
| Switch | Settings → Logs → "Detailed logging" turns INFO+DEBUG on/off **at runtime** through a `reload::Handle` on the filter. The state is saved in settings and applied at the next startup |
| Files | `logs/pos.YYYY-MM-DD.log`, daily rotation, keep 14 files (old ones removed at startup) |
| Format | One line per event: time, level, command span, fields. Human-readable |
| Spans | Each Tauri command runs in a span: `cmd{name=checkout_scan req=01HX…}` with the user id |
| Panics | Panic hook logs the panic and backtrace at ERROR before the default handler |
| UI errors | `window.onerror` and React error boundaries send to a `log_client_error` command |
| Export | "Export logs" → zip of the log files + `system.txt` (app version, OS, settings with secrets removed) |
| Never logged | Passwords, PINs, recovery codes, override tokens, full payment references (see [[Security Model]]) |

## What to log
- INFO: every command (name, duration, outcome), workflow milestones (sale completed: invoice, total), backups, migrations, hardware connect/disconnect.
- DEBUG: SQL timing for slow queries (>50 ms is logged at WARN), printer bytes length, scale raw lines.
- WARN: slow queries, retries, a missing second backup folder, near-full disk.
- ERROR: failed transactions, hardware errors, backup failures, integrity check failures.
