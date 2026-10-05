import type { SettingsSnapshot } from "../../../generated/SettingsSnapshot";

// The settings a new shop starts with, as Rust's defaults give them.
export function defaultSettings(): SettingsSnapshot {
  return {
    language: { language: "en" },
    shop_profile: {
      name: "",
      address_lines: [],
      phone: "",
      email: "",
      tax_reg_label: "BIN",
      tax_reg_no: "",
      logo_path: null,
      receipt_footer: [],
    },
    currency: {
      code: "BDT",
      symbol: "৳",
      symbol_before: true,
      minor_digits: 2,
      decimal_sep: ".",
      group_sep: ",",
      grouping: "indian",
    },
    rounding: { increment_minor: 0, mode: "nearest", applies_to: "cash_only" },
    tax: { label: "VAT", mode: "inclusive", default_rate_id: null },
    till: { till_id: null, till_code: "T1", business_day_cutoff_hour: 4 },
    receipt: {
      paper_width: "mm80",
      show_logo: true,
      show_tax_breakdown: true,
      show_tax_reg_no: true,
      show_invoice_barcode: true,
      show_customer_balance: true,
      copies: 1,
      header_lines: [],
      footer_lines: [],
    },
    hardware: { receipt_printer: null, drawer_pin: "pin2", scale: null, label_printer: null },
    scale_barcode_rules: [],
    internal_barcode: { prefix: "200" },
    backup: { primary_dir: "", secondary_dir: null, interval_min: 60, keep_last: 50 },
    logging: { detailed: false },
    checkout: { merge_scans: true, idle_lock_min: 5, blind_close: true, sounds: true },
    inventory: {
      allow_negative: true,
      near_expiry_days: 30,
      expired_policy: "warn",
      cost_method: "moving_average",
    },
    loyalty: {
      enabled: false,
      spend_unit_minor: 10000,
      points_per_unit: 1,
      value_per_point_minor: 100,
      min_redeem: 0,
    },
    credit: { enabled: false, default_limit_minor: 0 },
    denominations: [],
  };
}
