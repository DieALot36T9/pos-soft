// What the user reads for each error code that Rust sends (AppError).
// Each message says what happened and what to do. Never technical words.
// {{name}} is replaced by the error's details, for example {{max_decimals}} (i18next interpolation).
export const errors = {
  money_empty: "Type an amount. Example: 120.00",
  money_not_a_number: "Type the amount as a plain number. Example: 120.00",
  money_too_many_decimals: "Use at most {{max_decimals}} digits after the point.",
  money_too_large: "This amount is too large. Check it and try again.",
  id_invalid: "This item could not be found. Refresh the screen and try again.",
  date_invalid: "Type the date as year-month-day. Example: 2026-03-01",
  not_found: "This item no longer exists. Refresh the screen and try again.",
  permission_denied: "Your role can't do this. Ask an admin.",
  override_required: "This needs an admin's approval.",
  weight_invalid: "Type the weight as a plain number with at most 3 decimals. Example: 1.250",
  logo_unreadable: "This picture can't be used as a logo. Choose a PNG or JPEG file.",
  qr_text_too_long: "The QR code text is too long. Make it shorter.",
  // Setup and settings (SET-04 to SET-17).
  shop_name_missing: "Type the shop's name. Example: Rahim Store",
  email_invalid: "Type a full email address. Example: shop@example.com",
  currency_code_unknown: "Pick a currency from the list.",
  currency_decimals_unsupported:
    "This currency has more than 3 decimals, which the app doesn't support.",
  minor_digits_out_of_range: "Use 0 to 3 digits after the point.",
  minor_digits_locked:
    "The number of decimals can't change after the first sale, because saved amounts would change.",
  decimal_sep_missing: "Type the sign used before the decimals. Example: .",
  separator_invalid: "A separator can't be a digit or a minus sign.",
  separators_same: "The decimal sign and the thousands sign must be different.",
  group_sep_too_long: "Use a short thousands sign. Example: ,",
  rounding_step_negative: "The rounding step can't be below zero. Use 0 to turn rounding off.",
  tax_label_missing: "Type the tax name. Example: VAT",
  tax_rate_missing: "Add at least one tax rate first.",
  tax_default_rate_unknown: "Pick the default rate from the list.",
  tax_rate_name_missing: "Type the rate's name. Example: Standard 15%",
  tax_rate_name_taken: "Another rate already has this name. Pick a different name.",
  tax_rate_out_of_range: "Type a percentage from 0 to 100, with at most 2 decimals. Example: 7.5",
  tax_component_name_missing: "Every part of a rate needs a name. Example: CGST",
  tax_components_do_not_add_up: "The parts must add up to the whole rate.",
  till_code_invalid: "Use 1 to 6 letters or digits. Example: T1",
  cutoff_hour_out_of_range: "Pick an hour from 0 to 23.",
  receipt_copies_out_of_range: "Print 1 to 3 copies.",
  backup_interval_out_of_range: "Back up every 1 to 1440 minutes (one day).",
  backup_keep_out_of_range: "Keep at least 1 backup.",
  folder_not_writable: "The app can't save files in this folder. Pick another folder.",
  setup_already_done: "Setup is already finished. Change these in Settings.",
  // The first admin (SET-08).
  username_missing: "Type a username. Example: owner",
  username_in_use: "This username is taken. Pick another one.",
  display_name_missing: "Type the name shown on screen. Example: Rahim Uddin",
  password_too_short: "Use at least 8 characters for the password.",
  passwords_differ: "The two passwords are different. Type the same password twice.",
  admin_already_created: "The admin was already created. Go to the next step.",
  // Users, roles, login and approvals (USR-01 to USR-09).
  role_name_missing: "Type the role's name. Example: Supervisor",
  role_name_in_use: "Another role already has this name. Pick a different name.",
  max_discount_too_high: "The discount limit can be at most 100%.",
  admin_role_fixed: "The Admin role always has every permission and can't be changed.",
  pin_format: "Use 4 to 8 digits for the PIN. Example: 4821",
  secret_missing: "Give the user a PIN, a password, or both.",
  last_active_admin:
    "This is the only active admin. Make another user an admin first, then try again.",
  user_inactive: "This user is disabled. Ask an admin to enable them.",
  override_user_unknown: "There is no active user with this username.",
  recovery_code_wrong: "The recovery code is wrong. Check the paper copy and type it again.",
  recovery_user_not_admin: "Type the username of an active admin.",
  not_signed_in: "Nobody is logged in. Log in first.",
  till_locked: "The till is locked. Unlock it first.",
  till_not_locked: "The till isn't locked.",
  setup_not_done: "Finish setup first.",
  io_file_not_saved:
    "The file could not be saved. Pick another folder, or close the file in Excel.",
  // Backups (BAK-01 to BAK-06).
  backup_second_folder_failed:
    "The backup was saved, but not copied to the second folder. Check the USB drive or folder.",
  backup_check_failed:
    "The last backup failed its check. Make a new backup and tell the shop owner.",
  database_damaged: "The shop's data is damaged. Restore a backup first.",
  database_not_damaged: "The shop's data is fine, so there is nothing to recover.",
  hardware: "A device isn't working. Check that it's switched on and connected.",
  // Device errors, by the code Rust sends (AppError::Hardware). {{device}} is "printer" or "scale".
  hardware_timeout:
    "The {{device}} didn't answer in time. Check that it's switched on, then try again.",
  hardware_not_connected:
    "The {{device}} isn't connected. Check the cable or network, then try again.",
  hardware_send_failed: "Sending to the {{device}} stopped part way. Check it, then try again.",
  hardware_refused: "The {{device}} refused the job. Check its paper and cover, then try again.",
  hardware_no_weight: "The scale sent data without a weight. Check the scale settings.",
  hardware_bad_settings: "The {{device}} settings are wrong. Check them in Settings.",
  internal: "Something went wrong (ref {{ref}}). Copy the details and tell the shop owner.",
  unknown: "Something went wrong. Try again.",
};

export type ErrorText = typeof errors;
