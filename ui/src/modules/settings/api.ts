import type { CurrencyFormat } from "../../generated/CurrencyFormat";
import type { RateWithComponents } from "../../generated/RateWithComponents";
import type { SettingsChange } from "../../generated/SettingsChange";
import type { SettingsSnapshot } from "../../generated/SettingsSnapshot";
import { call } from "../../shared/bridge";

// Every setting at once.
export const getSettings = () => call<SettingsSnapshot>("settings_get");
// Saves one section. Rust checks it, audits it and returns every setting again.
export const saveSettings = (change: SettingsChange) =>
  call<SettingsSnapshot>("settings_save", { change });
// Detailed logging on or off, at once (BAK-07).
export const setDetailedLogging = (detailed: boolean) =>
  call<null>("logs_set_detailed", { detailed });
// How 1234567.5 looks in this format, written by Rust.
export const moneyPreview = (format: CurrencyFormat) => call<string>("money_preview", { format });
// The active tax rates, for the default rate list.
export const listRates = () =>
  call<RateWithComponents[]>("tax_rate_list", { includeInactive: false });
