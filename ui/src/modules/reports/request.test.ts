import { describe, expect, it } from "vitest";

import { buildRequest, type ReportForm } from "./request";

const form: ReportForm = {
  kind: "sales_by",
  from: "2026-03-01",
  to: "2026-03-31",
  tillId: "",
  shiftId: "",
  salesBy: "category",
  group: "product",
  categoryId: "",
  brandId: "B1",
  cashierId: "",
  top: "10",
  days: "30",
  supplierId: "",
  customerId: "",
};

describe("building a report request", () => {
  it("sends the range, filters and top N; empty filters mean all", () => {
    expect(buildRequest(form)).toEqual({
      ok: true,
      request: {
        report: "sales_by",
        by: "category",
        from: "2026-03-01",
        to: "2026-03-31",
        category_id: null,
        brand_id: "B1",
        cashier_id: null,
        top: 10,
      },
    });
  });

  it("refuses a day that isn't year-month-day, and a top that isn't a whole number", () => {
    expect(buildRequest({ ...form, from: "1/3/2026" })).toEqual({
      ok: false,
      problem: "dateInvalid",
    });
    expect(buildRequest({ ...form, top: "1.5" })).toEqual({ ok: false, problem: "topInvalid" });
  });

  it("needs a shift, supplier or customer for those reports", () => {
    expect(buildRequest({ ...form, kind: "shift" })).toEqual({ ok: false, problem: "pickShift" });
    expect(buildRequest({ ...form, kind: "supplier_statement" })).toEqual({
      ok: false,
      problem: "pickSupplier",
    });
    expect(buildRequest({ ...form, kind: "customer_statement", customerId: "C1" })).toEqual({
      ok: true,
      request: {
        report: "customer_statement",
        customer_id: "C1",
        from: "2026-03-01",
        to: "2026-03-31",
      },
    });
  });

  it("uses the first day for the daily summary, and needs no days for stock value", () => {
    expect(buildRequest({ ...form, kind: "daily_summary", tillId: "T1" })).toEqual({
      ok: true,
      request: { report: "daily_summary", date: "2026-03-01", till_id: "T1" },
    });
    expect(buildRequest({ ...form, kind: "stock_value", from: "" })).toEqual({
      ok: true,
      request: { report: "stock_value", by: "product" },
    });
  });
});
