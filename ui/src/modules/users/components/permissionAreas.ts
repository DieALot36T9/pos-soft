import type { Permission } from "../../../generated/Permission";

type Area =
  "checkout" | "returns" | "shifts" | "customers" | "stock" | "products" | "purchasing" | "office";

// Which group each permission's checkbox is shown in on the role form (Back Office Screens).
const areaOf: Record<Permission, Area> = {
  checkout_sell: "checkout",
  checkout_cancel_cart: "checkout",
  checkout_hold_resume: "checkout",
  checkout_manual_weight: "checkout",
  discount_apply: "checkout",
  price_override: "checkout",
  price_list_switch: "checkout",
  sell_on_credit: "checkout",
  credit_limit_exceed: "checkout",
  receipt_reprint_last: "checkout",
  receipt_reprint_any: "checkout",
  sale_void: "returns",
  return_with_receipt: "returns",
  return_without_receipt: "returns",
  shift_open_close_own: "shifts",
  shift_cash_in_out: "shifts",
  shift_no_sale_drawer: "shifts",
  shift_force_close: "shifts",
  customer_view_create: "customers",
  customer_manage: "customers",
  customer_receive_payment: "customers",
  loyalty_adjust: "customers",
  stock_view: "stock",
  stock_adjust: "stock",
  stock_count_enter: "stock",
  stock_count_manage: "stock",
  catalog_manage: "products",
  catalog_price_edit: "products",
  pricing_manage: "products",
  tax_manage: "products",
  labels_print: "products",
  purchasing_manage: "purchasing",
  purchasing_pay: "purchasing",
  reports_view_all: "office",
  data_import: "office",
  data_export: "office",
  users_manage: "office",
  settings_manage: "office",
  backup_run: "office",
  backup_restore: "office",
  audit_view: "office",
  logs_manage: "office",
};

const areaOrder: Area[] = [
  "checkout",
  "returns",
  "shifts",
  "customers",
  "stock",
  "products",
  "purchasing",
  "office",
];

// The permissions Rust lists, grouped by area, each group in Rust's order.
export function byArea(all: Permission[]): { area: Area; permissions: Permission[] }[] {
  return areaOrder
    .map((area) => ({ area, permissions: all.filter((each) => areaOf[each] === area) }))
    .filter((group) => group.permissions.length > 0);
}
