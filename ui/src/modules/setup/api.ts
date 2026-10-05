import type { AdminCreated } from "../../generated/AdminCreated";
import type { AdminStep } from "../../generated/AdminStep";
import type { BackupSettings } from "../../generated/BackupSettings";
import type { CurrencyChoice } from "../../generated/CurrencyChoice";
import type { CurrencyFormat } from "../../generated/CurrencyFormat";
import type { HardwareStep } from "../../generated/HardwareStep";
import type { MoneyStep } from "../../generated/MoneyStep";
import type { RateInput } from "../../generated/RateInput";
import type { RateWithComponents } from "../../generated/RateWithComponents";
import type { SettingsSnapshot } from "../../generated/SettingsSnapshot";
import type { SetupStatus } from "../../generated/SetupStatus";
import type { ShopProfile } from "../../generated/ShopProfile";
import type { TaxStep } from "../../generated/TaxStep";
import type { TillStep } from "../../generated/TillStep";
import { call } from "../../shared/bridge";

// Every call the setup wizard makes into Rust. Each step is saved by Rust in one
// transaction, so the wizard can stop and carry on later (SET-02).
export const getStatus = () => call<SetupStatus>("setup_status");
// What is saved so far, to fill in the steps again when the wizard resumes.
export const getSaved = () => call<SettingsSnapshot>("settings_get");
export const saveShop = (shop: ShopProfile) => call<SetupStatus>("setup_save_shop", { shop });
export const saveMoney = (step: MoneyStep) => call<SetupStatus>("setup_save_money", { step });
export const saveTax = (step: TaxStep) => call<SetupStatus>("setup_save_tax", { step });
export const saveAdmin = (step: AdminStep) => call<AdminCreated>("setup_save_admin", { step });
export const saveTill = (step: TillStep) => call<SetupStatus>("setup_save_till", { step });
export const saveHardware = (step: HardwareStep) =>
  call<SetupStatus>("setup_save_hardware", { step });
export const saveBackup = (step: BackupSettings) =>
  call<SetupStatus>("setup_save_backup", { step });
export const finish = () => call<SetupStatus>("setup_finish");
// Checks the file, then restarts the app with it (moving to a new computer).
export const restoreBackup = (file: string) => call<null>("setup_restore_backup", { file });
export const currencyChoices = () => call<CurrencyChoice[]>("currency_choices");
export const currencyFormatFor = (code: string) =>
  call<CurrencyFormat>("currency_format_for", { code });
// How 1234567.5 looks in this format, written by Rust.
export const moneyPreview = (format: CurrencyFormat) => call<string>("money_preview", { format });
export const listRates = () =>
  call<RateWithComponents[]>("tax_rate_list", { includeInactive: false });
export const addRate = (input: RateInput) => call<string>("tax_rate_save", { input });
// Opens the print dialog for the recovery code.
export const printWindow = () => call<null>("print_window");
