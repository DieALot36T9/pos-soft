// Screen text for the admin approval dialog (USR-05), in plain English (ADR-022).
export const approval = {
  title: "This needs an admin: {{action}}",
  intro: "An admin types their password to allow this one action. It works once.",
  admin: { label: "Admin", tip: "Choose the admin who allows this action." },
  password: { label: "Admin password", tip: "The admin's password. A PIN doesn't work here." },
  approve: { label: "Allow", tip: "Check the password and do the action once." },
  cancel: { label: "Cancel", tip: "Don't do the action." },
  noAdmins: "There is no user with a password who could allow this.",
  wrong: "The password is wrong. {{left}} tries left before a short lock.",
  locked: "Too many wrong tries for this admin. Try again in {{seconds}} seconds.",
  notAllowed: "This user may not allow this action either. Choose another admin.",
};

export type ApprovalText = typeof approval;
