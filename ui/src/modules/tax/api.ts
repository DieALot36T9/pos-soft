import type { RateInput } from "../../generated/RateInput";
import type { RateWithComponents } from "../../generated/RateWithComponents";
import { call } from "../../shared/bridge";

// The shop's tax rates, in list order.
export const listRates = (includeInactive: boolean) =>
  call<RateWithComponents[]>("tax_rate_list", { includeInactive });
// Adds a rate (no id) or changes one. Rust checks the name and percentage.
export const saveRate = (input: RateInput) => call<string>("tax_rate_save", { input });
// Makes a rate inactive. Rates are never deleted (TAX-10).
export const deactivateRate = (id: string) => call<null>("tax_rate_deactivate", { id });
