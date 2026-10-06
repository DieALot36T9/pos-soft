// All screen text for the login, lock and "My PIN and password" screens, in plain English (ADR-022).
export const en = {
  login: {
    title: "Log in",
    intro: "Choose your name.",
    tileTip: "Log in as {{name}}.",
    noUsers: "No users can log in. Use the recovery code to set an admin password.",
    pin: { label: "PIN", tip: "Your PIN: 4 to 8 digits. They are hidden." },
    password: { label: "Password", tip: "Your password. Letters are hidden." },
    either: {
      label: "PIN or password",
      tip: "Type your PIN or your password. They are hidden.",
    },
    hello: "Hello, {{name}}",
    button: { label: "Log in", tip: "Check the PIN or password and open the app." },
    back: { label: "Back", tip: "Choose a different name." },
    wrong: "That is wrong. {{left}} tries left before a short lock.",
    locked: "Too many wrong tries. Try again in {{seconds}} seconds.",
    forgot: {
      label: "Forgot admin password",
      tip: "Set a new admin password with the recovery code from setup.",
    },
  },
  recovery: {
    title: "Forgot admin password",
    intro:
      "Type the recovery code from the paper you kept at setup. A new code is shown afterwards; the old one stops working.",
    code: {
      label: "Recovery code",
      tip: "The 20 letters and digits. Example: 7KQ2-M9XD-4RTW-PZ8C-H3NB",
    },
    username: { label: "Admin username", tip: "The username of the admin. Example: owner" },
    password: { label: "New password", tip: "At least 8 characters. Letters are hidden." },
    passwordAgain: { label: "New password again", tip: "Type the same new password again." },
    save: { label: "Set password", tip: "Check the code and set the new password." },
    back: { label: "Back", tip: "Go back to the login screen. Nothing is changed." },
    doneTitle: "The new password is set",
    doneText:
      "Write this new recovery code on paper and keep it safe. It is shown only now. The old code no longer works.",
    doneButton: { label: "I wrote it down", tip: "Go back to the login screen." },
  },
  lock: {
    title: "Till locked",
    intro: "{{name}} locked the till. Type the PIN or password to go on.",
    secret: { label: "PIN or password", tip: "The PIN or password of {{name}}. It is hidden." },
    unlock: { label: "Unlock", tip: "Check the PIN or password and go back to work." },
    wrong: "That is wrong. {{left}} tries left before a short lock.",
    locked: "Too many wrong tries. Try again in {{seconds}} seconds.",
    adminTitle: "An admin can unlock to help",
    adminUsername: { label: "Admin username", tip: "The admin's username. Example: owner" },
    adminPassword: { label: "Admin password", tip: "The admin's password. A PIN doesn't work." },
    adminUnlock: {
      label: "Unlock as admin",
      tip: "Unlock for {{name}}. The audit log records the admin's name.",
    },
    adminNotAllowed: "This user isn't an admin.",
    switchUser: {
      label: "Switch user",
      tip: "Log {{name}} out, so someone else can log in.",
    },
  },
  account: {
    title: "My PIN and password",
    intro: "Change your own PIN or password. Type the current one first.",
    kind: { label: "Change", tip: "Choose what to change." },
    kinds: { pin: "PIN", password: "Password" },
    current: {
      label: "Current PIN or password",
      tip: "Your PIN or password as it is now. It is hidden.",
    },
    newPin: { label: "New PIN", tip: "4 to 8 digits. Example: 4821" },
    newPassword: { label: "New password", tip: "At least 8 characters." },
    again: { label: "New one again", tip: "Type the same new PIN or password again." },
    differ: "The two new ones are different. Type the same one twice.",
    save: { label: "Save", tip: "Check the current one and save the new one." },
    saved: "Saved. Use the new one next time you log in.",
    wrong: "The current PIN or password is wrong. {{left}} tries left before a short lock.",
    locked: "Too many wrong tries. Try again in {{seconds}} seconds.",
  },
};

export type AuthText = typeof en;
