// All screen text for the app frame, in plain English (ADR-022).
export const en = {
  title: "Welcome",
  intro: "The app is running.",
  checkButton: {
    label: "Check app",
    tip: "Ask the app which version is running.",
  },
  versionLabel: "Running version",
  loading: "Starting…",
  startProblem: "The app could not read its data. Close it and open it again.",
  menu: {
    label: "Main menu",
    home: { label: "Home", tip: "The start page with the app's version." },
    hardware: { label: "Printer and devices", tip: "Test print and the Preview printer." },
    settings: { label: "Settings", tip: "Shop details, money, tax, receipt, backups and logs." },
    tax: { label: "Tax rates", tip: "Add, change or deactivate tax rates." },
    backups: { label: "Backups", tip: "Back up now, see backups and restore one." },
    users: { label: "Users", tip: "Add users, reset a PIN or password, disable a user." },
    roles: { label: "Roles", tip: "What each role may do, and its biggest discount." },
    audit: { label: "Audit log", tip: "Every sensitive action: who, when and what." },
    account: { label: "My PIN and password", tip: "Change your own PIN or password." },
  },
  signedInAs: "Logged in: {{name}} ({{role}})",
  signOut: {
    label: "Log out",
    tip: "Log out of the app. The next person logs in with their tile.",
  },
  lock: { label: "Lock", tip: "Lock the till. Your PIN or password unlocks it." },
};

export type AppText = typeof en;
