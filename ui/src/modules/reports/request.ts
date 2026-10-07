import type { ReportGroup } from "../../generated/ReportGroup";
import type { ReportRequest } from "../../generated/ReportRequest";
import type { SalesBy } from "../../generated/SalesBy";
import type { ReportsText } from "./text/en";
import { isDay } from "./dates";

// Every report on the screen, in menu order, with its group (Back Office Screens).
export type ReportKind = ReportRequest["report"];
export type ReportGroupName = keyof ReportsText["list"]["groups"];

export const REPORT_LIST: { group: ReportGroupName; kinds: ReportKind[] }[] = [
  { group: "sales", kinds: ["daily_summary", "shift", "sales_by"] },
  { group: "profit", kinds: ["profit"] },
  { group: "stock", kinds: ["stock_value", "low_stock", "near_expiry", "stock_movement"] },
  { group: "tax", kinds: ["tax"] },
  {
    group: "money_owed",
    kinds: ["payables_aging", "receivables_aging", "supplier_statement", "customer_statement"],
  },
  { group: "control", kinds: ["discounts_overrides"] },
];

// What each report asks for, so the screen shows only those fields.
export const NEEDS: Record<ReportKind, { range?: true; day?: true; group?: true }> = {
  daily_summary: { day: true },
  shift: {},
  sales_by: { range: true },
  profit: { range: true, group: true },
  stock_value: { group: true },
  low_stock: {},
  near_expiry: {},
  stock_movement: { range: true },
  tax: { range: true },
  payables_aging: {},
  receivables_aging: {},
  supplier_statement: { range: true },
  customer_statement: { range: true },
  discounts_overrides: { range: true },
};

// Everything typed or picked on the form. Empty text means "All" or "not chosen".
export type ReportForm = {
  kind: ReportKind;
  from: string;
  to: string;
  tillId: string;
  shiftId: string;
  salesBy: SalesBy;
  group: ReportGroup;
  categoryId: string;
  brandId: string;
  cashierId: string;
  top: string;
  days: string;
  supplierId: string;
  customerId: string;
};

type Problem = keyof ReportsText["problems"];

// The request for Rust, or the reason it can't be sent yet. Only checks what was typed;
// Rust checks the rest (a backwards range, an unknown supplier).
export function buildRequest(
  form: ReportForm,
): { ok: true; request: ReportRequest } | { ok: false; problem: Problem } {
  const needs = NEEDS[form.kind];
  const from = form.from.trim();
  const to = form.to.trim();
  if ((needs.range && (!isDay(from) || !isDay(to))) || (needs.day && !isDay(from))) {
    return { ok: false, problem: "dateInvalid" };
  }
  const orNull = (text: string) => (text === "" ? null : text);
  const done = (request: ReportRequest) => ({ ok: true as const, request });
  switch (form.kind) {
    case "daily_summary":
      return done({ report: "daily_summary", date: from, till_id: orNull(form.tillId) });
    case "shift":
      return form.shiftId === ""
        ? { ok: false, problem: "pickShift" }
        : done({ report: "shift", shift_id: form.shiftId });
    case "sales_by": {
      const top = form.top.trim();
      if (top !== "" && !/^[1-9]\d{0,5}$/.test(top)) {
        return { ok: false, problem: "topInvalid" };
      }
      return done({
        report: "sales_by",
        by: form.salesBy,
        from,
        to,
        category_id: orNull(form.categoryId),
        brand_id: orNull(form.brandId),
        cashier_id: orNull(form.cashierId),
        top: top === "" ? null : Number(top),
      });
    }
    case "profit":
      return done({ report: "profit", by: form.group, from, to });
    case "stock_value":
      return done({ report: "stock_value", by: form.group });
    case "low_stock":
    case "payables_aging":
    case "receivables_aging":
      return done({ report: form.kind });
    case "near_expiry": {
      const days = form.days.trim();
      if (!/^\d{1,4}$/.test(days)) {
        return { ok: false, problem: "daysInvalid" };
      }
      return done({ report: "near_expiry", days: Number(days) });
    }
    case "stock_movement":
    case "tax":
    case "discounts_overrides":
      return done({ report: form.kind, from, to });
    case "supplier_statement":
      return form.supplierId === ""
        ? { ok: false, problem: "pickSupplier" }
        : done({ report: "supplier_statement", supplier_id: form.supplierId, from, to });
    case "customer_statement":
      return form.customerId === ""
        ? { ok: false, problem: "pickCustomer" }
        : done({ report: "customer_statement", customer_id: form.customerId, from, to });
  }
}
