---
tags: [runbook]
---
# Backup and Restore

How backups work: [[Database Strategy#Backups]]. Requirements: [[Req - Backup and Logging]].

## Daily routine (shop owner)
- Keep the **USB drive** plugged in (second backup folder). Swap it with a second USB drive weekly and keep one at home.
- Check the dashboard: a red "Backup problem" card means backups are failing. Act the same day.

## Make a backup now
Back office → Backups → **Back up now** (optionally pick a folder).

## Restore
Use this when the database is damaged, or a big mistake needs undoing (e.g. a wrong import).
1. Admin: Back office → Backups. Find the backup **just before** the problem (date, reason, last invoice number help).
2. Click **Restore** and confirm the date shown.
3. The app backs up the current state (`pre-restore`), restores, and restarts.
4. Check the last invoice number and today's sales.
**Everything after the backup's time is lost**, so compare with paper receipts if needed.

## The app says "Database check failed" at startup
1. Don't reinstall. Choose **Restore latest good backup** on the recovery screen.
2. If no backup passes the check, copy the whole app data folder to USB and contact support with it (and the logs).

## Where files are
Windows: `%APPDATA%\<app-id>\` → `pos.sqlite`, `backups\`, `images\`, `logs\`.
