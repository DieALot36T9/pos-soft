import type { AdminCreated } from "../../../generated/AdminCreated";
import type { AdminStep } from "../../../generated/AdminStep";
import type { AppError } from "../../../generated/AppError";
import type { BackupDone } from "../../../generated/BackupDone";
import type { BackupRow } from "../../../generated/BackupRow";
import type { BackupSettings } from "../../../generated/BackupSettings";
import type { CurrencyChoice } from "../../../generated/CurrencyChoice";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { HardwareStep } from "../../../generated/HardwareStep";
import type { MoneyStep } from "../../../generated/MoneyStep";
import type { RateInput } from "../../../generated/RateInput";
import type { RateWithComponents } from "../../../generated/RateWithComponents";
import type { SettingsChange } from "../../../generated/SettingsChange";
import type { SettingsSnapshot } from "../../../generated/SettingsSnapshot";
import type { SetupStatus } from "../../../generated/SetupStatus";
import type { SetupStep } from "../../../generated/SetupStep";
import type { ShopProfile } from "../../../generated/ShopProfile";
import type { StartupView } from "../../../generated/StartupView";
import type { TaxStep } from "../../../generated/TaxStep";
import type { TillStep } from "../../../generated/TillStep";
import { catalogHandlers, connectCatalog, resetCatalog } from "./catalog";
import { defaultSettings } from "./defaultSettings";
import {
  createFirstAdmin,
  currentSession,
  peopleHandlers,
  recordFakeEvent,
  requireOrApproval,
  requirePermission,
  resetPeople,
} from "./people";
import { connectPrinter, resetPrinter } from "./printer";
import { connectStockToShop, resetStock, stockHandlers } from "./stock";
import { connectTill, resetTill, tillHandlers } from "./till";
import { connectCustomersToShop, customerHandlers, resetCustomers } from "./customers";
import { connectPurchasingToShop, purchasingHandlers, resetPurchasing } from "./purchasing";
import { connectIoToShop, ioHandlers } from "./io";
import { connectReportsToShop, reportHandlers } from "./reports";

// A pretend shop for the fake bridge. Rust does the real checks; this stand-in
// checks only what the screens and their tests need, with the same error codes.

const wizardOrder: SetupStep[] = [
  "shop",
  "currency",
  "tax",
  "admin",
  "till",
  "hardware",
  "backup",
  "products",
];

let completed = new Set<SetupStep>();
let snapshot: SettingsSnapshot = defaultSettings();
let rates: RateWithComponents[] = [];
let backups: BackupRow[] = [];

// Starts the pretend shop again: empty, or set up (and then signed in or not).
// Tests call it before each test.
export function resetFakeShop(startWith: "fresh" | "set_up" = "fresh", signedIn = false) {
  completed = new Set();
  snapshot = defaultSettings();
  rates = [];
  backups = [];
  resetPeople(startWith === "set_up", signedIn);
  resetCatalog(startWith === "set_up");
  resetStock(startWith === "set_up");
  resetTill();
  resetCustomers(startWith === "set_up");
  resetPurchasing(startWith === "set_up");
  resetPrinter();
  if (startWith === "set_up") {
    completed = new Set(wizardOrder.filter((step) => step !== "hardware"));
    snapshot.shop_profile.name = "Rahim Store";
    rates = [rate("01FAKERATE0000000000000001", "Standard 15%", 1500)];
    snapshot.tax.default_rate_id = rates[0]?.id ?? null;
    backups = [backupRow("pos-20261004-200000-close.sqlite", "2026-10-04 20:00:00", "close")];
  }
}

function rate(id: string, name: string, bps: number): RateWithComponents {
  return { id, name, rate_bps: bps, active: true, sort: 0, components: [] };
}

function backupRow(fileName: string, takenAt: string, reason: string): BackupRow {
  return {
    file: `/fake/backups/${fileName}`,
    file_name: fileName,
    taken_at: takenAt,
    reason,
    size_bytes: 245760,
    usable: true,
    sale_count: 0,
    last_invoice_no: null,
  };
}

function refuse(field: string | null, code: string): never {
  const error: AppError = { kind: "validation", field, code, params: {} };
  // Rust's invoke fails with a plain AppError object, so the fake does too.
  // eslint-disable-next-line @typescript-eslint/only-throw-error
  throw error;
}

function conflict(code: string): never {
  const error: AppError = { kind: "conflict", code, params: {} };
  // eslint-disable-next-line @typescript-eslint/only-throw-error
  throw error;
}

function setupStatus(): SetupStatus {
  const done = wizardOrder.filter((step) => completed.has(step));
  return {
    completed: done,
    next_step: wizardOrder.find((step) => !completed.has(step)) ?? null,
    is_complete: wizardOrder.every((step) => step === "hardware" || completed.has(step)),
  };
}

function requireSetupOpen() {
  if (setupStatus().is_complete) {
    conflict("setup_already_done");
  }
}

// Settings and tax commands work during setup, then need a signed-in admin.
function requireAdminOrSetup(permission: "settings_manage" | "tax_manage") {
  if (setupStatus().is_complete) {
    requirePermission(permission);
  }
}

function step(name: SetupStep, save: () => void): SetupStatus {
  requireSetupOpen();
  save();
  completed.add(name);
  return setupStatus();
}

// The catalog reads the shop's tax rates and money format.
connectStockToShop(() => snapshot.currency);
connectTill(
  () => snapshot.currency,
  () => snapshot,
);
connectCustomersToShop(() => snapshot.currency);
connectPurchasingToShop(() => snapshot.currency);
connectReportsToShop(() => ({ name: snapshot.shop_profile.name, currency: snapshot.currency }));
connectPrinter(() => snapshot.hardware);

connectIoToShop(() => snapshot.currency);

connectCatalog({
  rates: () => rates,
  defaultRate: () => snapshot.tax.default_rate_id,
  currency: () => snapshot.currency,
});

export const shopHandlers = {
  ...peopleHandlers,
  ...catalogHandlers,
  ...stockHandlers,
  ...tillHandlers,
  ...customerHandlers,
  ...purchasingHandlers,
  ...reportHandlers,
  ...ioHandlers,
  startup_status: (): StartupView => ({
    damaged: false,
    setup: setupStatus(),
    session: currentSession(),
    backup_warning: null,
    idle_lock_min: snapshot.checkout.idle_lock_min,
  }),
  setup_status: (): SetupStatus => setupStatus(),
  setup_save_shop: (args?: Record<string, unknown>): SetupStatus => {
    const shop = args?.shop as ShopProfile;
    if (shop.name.trim() === "") {
      refuse("name", "shop_name_missing");
    }
    return step("shop", () => (snapshot.shop_profile = shop));
  },
  setup_save_money: (args?: Record<string, unknown>): SetupStatus => {
    const money = args?.step as MoneyStep;
    return step("currency", () => {
      snapshot.currency = money.currency;
      snapshot.rounding = money.rounding;
    });
  },
  setup_save_tax: (args?: Record<string, unknown>): SetupStatus => {
    const tax = args?.step as TaxStep;
    const active = rates.filter((each) => each.active);
    if (active.length === 0) {
      refuse("rates", "tax_rate_missing");
    }
    return step("tax", () => {
      snapshot.tax = { ...tax, default_rate_id: tax.default_rate_id ?? active[0]?.id ?? null };
    });
  },
  setup_save_admin: (args?: Record<string, unknown>): AdminCreated => {
    requireSetupOpen();
    const form = args?.step as AdminStep;
    if (form.password.length < 8) {
      refuse("password", "password_too_short");
    }
    if (form.password !== form.password_again) {
      refuse("password_again", "passwords_differ");
    }
    const created = createFirstAdmin(form.username, form.display_name, form.password);
    completed.add("admin");
    return created as AdminCreated;
  },
  setup_save_till: (args?: Record<string, unknown>): SetupStatus => {
    const till = args?.step as TillStep;
    if (!/^[A-Za-z0-9]{1,6}$/.test(till.till_code.trim())) {
      refuse("till_code", "till_code_invalid");
    }
    return step("till", () => {
      snapshot.till = {
        till_id: "01FAKETILL0000000000000001",
        till_code: till.till_code.trim().toUpperCase(),
        business_day_cutoff_hour: till.business_day_cutoff_hour,
      };
    });
  },
  setup_save_hardware: (args?: Record<string, unknown>): SetupStatus => {
    const hardware = args?.step as HardwareStep;
    return step("hardware", () => {
      snapshot.hardware.receipt_printer = hardware.receipt_printer;
      snapshot.hardware.drawer_pin = hardware.drawer_pin;
      snapshot.receipt.paper_width = hardware.paper_width;
    });
  },
  setup_save_backup: (args?: Record<string, unknown>): SetupStatus => {
    const backup = args?.step as BackupSettings;
    return step("backup", () => (snapshot.backup = backup));
  },
  setup_finish: (): SetupStatus => step("products", () => undefined),
  setup_restore_backup: (): null => {
    requireSetupOpen();
    resetFakeShop("set_up");
    return null;
  },
  currency_choices: (): CurrencyChoice[] => [
    { code: "BDT", name: "Bangladeshi taka", minor_digits: 2, symbol: "৳" },
    { code: "EUR", name: "Euro", minor_digits: 2, symbol: "€" },
    { code: "INR", name: "Indian rupee", minor_digits: 2, symbol: "₹" },
    { code: "JPY", name: "Japanese yen", minor_digits: 0, symbol: "¥" },
    { code: "USD", name: "US dollar", minor_digits: 2, symbol: "$" },
  ],
  currency_format_for: (args?: Record<string, unknown>): CurrencyFormat => {
    const code = String(args?.code);
    const choice = shopHandlers.currency_choices().find((each) => each.code === code);
    if (!choice) {
      refuse("code", "currency_code_unknown");
    }
    const indian = ["BDT", "INR"].includes(code);
    return {
      code,
      symbol: choice.symbol,
      symbol_before: true,
      minor_digits: choice.minor_digits,
      decimal_sep: ".",
      group_sep: ",",
      grouping: indian ? "indian" : "western",
    };
  },
  settings_get: (): SettingsSnapshot => {
    requireAdminOrSetup("settings_manage");
    return snapshot;
  },
  settings_save: (args?: Record<string, unknown>): SettingsSnapshot => {
    requireAdminOrSetup("settings_manage");
    const change = args?.change as SettingsChange;
    applyChange(change);
    return snapshot;
  },
  logs_set_detailed: (args?: Record<string, unknown>): null => {
    requirePermission("logs_manage");
    snapshot.logging = { detailed: Boolean(args?.detailed) };
    return null;
  },
  tax_rate_list: (args?: Record<string, unknown>): RateWithComponents[] => {
    requireAdminOrSetup("tax_manage");
    return rates.filter((each) => each.active || args?.includeInactive === true);
  },
  tax_rate_save: (args?: Record<string, unknown>): string => {
    requireAdminOrSetup("tax_manage");
    const input = args?.input as RateInput;
    if (input.rate.name.trim() === "") {
      refuse("name", "tax_rate_name_missing");
    }
    const sameName = rates.find(
      (each) => each.name.toLowerCase() === input.rate.name.trim().toLowerCase(),
    );
    if (sameName && sameName.id !== input.id) {
      conflict("tax_rate_name_taken");
    }
    const id = input.id ?? `01FAKERATE${String(rates.length + 2).padStart(16, "0")}`;
    const saved = {
      ...rate(id, input.rate.name.trim(), input.rate.rate_bps),
      sort: input.rate.sort,
    };
    rates = [...rates.filter((each) => each.id !== id), saved];
    return id;
  },
  tax_rate_deactivate: (args?: Record<string, unknown>): null => {
    requireAdminOrSetup("tax_manage");
    rates = rates.map((each) => (each.id === args?.id ? { ...each, active: false } : each));
    return null;
  },
  backup_now: (): BackupDone => {
    requirePermission("backup_run");
    recordFakeEvent("backup.manual", currentSession()?.user_id ?? null);
    const row = backupRow(
      `pos-20261005-1400${String(backups.length).padStart(2, "0")}-manual.sqlite`,
      `2026-10-05 14:00:${String(backups.length).padStart(2, "0")}`,
      "manual",
    );
    backups = [row, ...backups];
    return { file: row.file, warning: null };
  },
  backup_list: (): BackupRow[] => {
    requirePermission("backup_run");
    return backups;
  },
  backup_restore: (args?: Record<string, unknown>): null => {
    requireOrApproval("backup_restore", args?.approval);
    return null;
  },
  recovery_backups: (): BackupRow[] => backups,
  recovery_restore: (): null => null,
};

function applyChange(change: SettingsChange) {
  switch (change.section) {
    case "shop":
      if (change.value.name.trim() === "") {
        refuse("name", "shop_name_missing");
      }
      snapshot.shop_profile = change.value;
      break;
    case "money":
      snapshot.currency = change.value.currency;
      snapshot.rounding = change.value.rounding;
      break;
    case "tax":
      snapshot.tax = change.value;
      break;
    case "till":
      snapshot.till = { ...snapshot.till, ...change.value };
      break;
    case "receipt":
      snapshot.receipt = change.value;
      break;
    case "backup":
      snapshot.backup = change.value;
      break;
    case "hardware":
      if (change.value.scale !== null && change.value.scale.port.trim() === "") {
        refuse("scale", "scale_port_missing");
      }
      snapshot.hardware = change.value;
      break;
    case "loyalty":
      snapshot.loyalty = change.value;
      break;
    case "credit":
      snapshot.credit = change.value;
      break;
    default:
      break;
  }
}

// `pnpm dev:mock` and the Playwright tests pick the start with `?shop=`:
// "fresh" (the setup wizard), "signed_out" (sign-in), anything else: set up and signed in.
const start = new URLSearchParams(window.location.search).get("shop");
resetFakeShop(start === "fresh" ? "fresh" : "set_up", start !== "signed_out");
