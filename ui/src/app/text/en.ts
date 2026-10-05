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
  },
  signedInAs: "Signed in: {{name}}",
  signOut: { label: "Sign out", tip: "Sign out of the app. The next person signs in again." },
  signIn: {
    title: "Sign in",
    intro: "Sign in with the admin account made during setup.",
    username: { label: "Username", tip: "The username from setup. Example: owner" },
    password: { label: "Password", tip: "Your password. Letters are hidden." },
    button: { label: "Sign in", tip: "Check the username and password and open the app." },
    refused: "The username or password is wrong.",
    refusedLeft: "The username or password is wrong. {{left}} tries left before a short lock.",
    locked: "Too many wrong tries. Wait a minute and try again.",
  },
};

export type AppText = typeof en;
