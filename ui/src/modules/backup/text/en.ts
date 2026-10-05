// All screen text for the backup screens, in plain English (ADR-022).
export const en = {
  page: {
    title: "Backups",
    intro: "The app backs up by itself regularly and when it closes. Each backup is checked.",
  },
  now: {
    label: "Back up now",
    tip: "Save a copy of all data now, for example before a big change.",
    busy: "Backing up…",
    done: "Backed up to {{file}}",
  },
  table: {
    takenAt: "When",
    reason: "Why",
    size: "Size",
    sales: "Sales",
    lastInvoice: "Last invoice",
    check: "Check",
    actions: "Actions",
    usable: "Checked",
    usableTip: "This backup passed its check and can be restored.",
    unusable: "Damaged",
    unusableTip: "This backup failed its check or is from a newer app. It can't be restored.",
    empty: "No backups yet.",
    sizeKb: "{{kb}} KB",
    none: "—",
  },
  reasons: {
    auto: "Timed",
    close: "App closed",
    manual: "By hand",
    "pre-restore": "Before a restore",
    "pre-upgrade": "Before an update",
    other: "{{reason}}",
  },
  restore: {
    tip: "Put this backup back. The app restarts. Today's data after this backup is lost, but a backup is made first.",
    confirmTitle: "Restore the backup from {{when}}?",
    confirmText:
      "Everything entered after {{when}} will be gone. The app first backs up the data as it is now, then restarts.",
    yes: { label: "Restore and restart", tip: "Restore this backup now. The app restarts." },
    no: { label: "Cancel", tip: "Don't restore. Nothing changes." },
    busy: "Restoring. The app will restart…",
  },
  recovery: {
    title: "The shop's data is damaged",
    intro:
      "The app checked its data at start and found damage. Restore the newest backup that passed its check. Nothing else can be used until then.",
    restoreNewest: {
      label: "Restore the newest backup",
      tip: "Put back the newest checked backup and restart the app.",
    },
    none: "No checked backup was found in the backup folder. Call for help before doing anything else.",
  },
  warning: {
    banner: "Backup problem: {{message}}",
  },
};

export type BackupText = typeof en;
