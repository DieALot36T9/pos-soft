---
tags: [requirements]
modules: [db, app, settings]
---
# Requirements: Backup and Logging (BAK)

Modules: [[Module - db]], [[Module - app]]. Runbook: [[Backup and Restore]]. Design: [[Database Strategy#Backups]], [[Errors and Logging]].

### BAK-01 Automatic backups
- When: every N minutes while the app is open (default 60, skipped if nothing changed), when the app closes, and **before**: a schema upgrade, a restore, an import, and posting a stock count
- How: SQLite `VACUUM INTO` a new file `pos-YYYYMMDD-HHMMSS-<reason>.sqlite`. It's never a plain file copy
- Keep the newest N (default 50), plus the last backup of each day for 30 days. Older ones are deleted

### BAK-02 Second backup folder
- If a second folder is set (e.g. a USB drive), each backup is also copied there. A missing or full folder shows a warning banner but never blocks work

### BAK-03 Checking backups
- Each new backup is opened read-only and passes `PRAGMA quick_check`. Failure → warning banner + error log

### BAK-04 Manual backup
- "Back up now" (anyone) with an optional note. Admins can choose a target folder

### BAK-05 Restore
- Admin only. The list shows backups with date, reason, size, number of sales and last invoice number
- Restoring: check the file → back up the current database (reason `pre-restore`) → swap files → restart the app. Audited

### BAK-06 Startup integrity check
- At startup, `PRAGMA quick_check` runs. If it fails, a recovery screen offers to restore the newest backup that passes the check. Nothing else can be used until this is resolved

### BAK-07 Logging switch
- **Errors and warnings are always logged.** The switch (Settings → Logs) turns **detailed** logging (info/debug) on or off immediately, without a restart
- Log files rotate daily and are kept for 14 days. They're in app data `logs/`
- "Export logs" saves a zip of the logs and the system info (app version, OS, settings with secrets removed) for support

### BAK-08 What is never logged
- Passwords, PINs, recovery codes, override tokens, full payment references longer than 4 characters (masked)
