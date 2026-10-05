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
