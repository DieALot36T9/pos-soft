// What the user reads for each error code that Rust sends (AppError).
// Each message says what happened and what to do. Never technical words.
// {name} is replaced by the error's details, for example {max_decimals}.
export const errors = {
  money_empty: "Type an amount. Example: 120.00",
  money_not_a_number: "Type the amount as a plain number. Example: 120.00",
  money_too_many_decimals: "Use at most {max_decimals} digits after the point.",
  money_too_large: "This amount is too large. Check it and try again.",
  id_invalid: "This item could not be found. Refresh the screen and try again.",
  date_invalid: "Type the date as year-month-day. Example: 2026-03-01",
  not_found: "This item no longer exists. Refresh the screen and try again.",
  permission_denied: "Your role can't do this. Ask an admin.",
  override_required: "This needs an admin's approval.",
  hardware: "A device isn't working. Check that it's switched on and connected.",
  internal: "Something went wrong (ref {ref}). Copy the details and tell the shop owner.",
  unknown: "Something went wrong. Try again.",
};

export type ErrorText = typeof errors;
