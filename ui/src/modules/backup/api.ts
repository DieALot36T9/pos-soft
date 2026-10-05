import type { BackupDone } from "../../generated/BackupDone";
import type { BackupRow } from "../../generated/BackupRow";
import { call } from "../../shared/bridge";

// Makes a checked backup now (BAK-04).
export const backupNow = () => call<BackupDone>("backup_now");
// The backups in the main folder, newest first (BAK-05).
export const listBackups = () => call<BackupRow[]>("backup_list");
// Restores a backup. The real app restarts and doesn't answer.
export const restoreBackup = (file: string) => call<null>("backup_restore", { file });
// The recovery screen's list and restore, for a damaged database (BAK-06).
export const recoveryBackups = () => call<BackupRow[]>("recovery_backups");
export const recoveryRestore = (file: string) => call<null>("recovery_restore", { file });
