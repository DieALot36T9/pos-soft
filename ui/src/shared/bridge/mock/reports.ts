import type { AppError } from "../../../generated/AppError";
import type { Cell } from "../../../generated/Cell";
import type { Choice } from "../../../generated/Choice";
import type { Column } from "../../../generated/Column";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { PrintResult } from "../../../generated/PrintResult";
import type { ReportChoices } from "../../../generated/ReportChoices";
import type { ReportRequest } from "../../../generated/ReportRequest";
import type { ReportTable } from "../../../generated/ReportTable";
import type { ShiftChoice } from "../../../generated/ShiftChoice";
import type { ShiftList } from "../../../generated/ShiftList";
import { currentSession, requirePermission } from "./people";
import { fakePrint } from "./printer";
import { testReceiptPicture } from "./testReceipt";

// A pretend Reports screen for the fake bridge: fixed demo reports in the real
// `ReportTable` shape. Rust works out the real numbers (Module - reports); these
// are only there so the screen and its flows can be tried.

type Args = Record<string, unknown>;

const TODAY = "2026-03-02";
let shop: () => { name: string; currency: CurrencyFormat } = () => {
  throw new Error("The fake reports aren't connected to the fake shop.");
};

// shop.ts calls this once, so the reports use the shop's name and money format.
export function connectReportsToShop(fromShop: () => { name: string; currency: CurrencyFormat }) {
  shop = fromShop;
}

const text = (value: string): Cell => ({ kind: "text", value });
const money = (value: number): Cell => ({ kind: "money", value });
const count = (value: number): Cell => ({ kind: "int", value });
const qty = (value: number): Cell => ({ kind: "qty", value });
const column = (key: string, title: string, kind: Column["kind"]): Column => ({ key, title, kind });

function table(
  title: string,
  request: ReportRequest,
  columns: Column[],
  rows: Cell[][],
): ReportTable {
  const from = "from" in request ? request.from : "date" in request ? request.date : null;
  const to = "to" in request ? request.to : "date" in request ? request.date : null;
  return { title, params: { from, to, filters: [] }, columns, rows, totals: null, sections: [] };
}

// RPT-10 in the fake: everything with ReportsViewAll, else only the user's own shifts.
function mayRun(request: ReportRequest) {
  const session = currentSession();
  if (session?.permissions.includes("reports_view_all")) {
    return;
  }
  if (request.report === "shift" && shifts().some((each) => each.id === request.shift_id)) {
    return;
  }
  requirePermission("reports_view_all");
}

// One closed shift of the signed-in user, and one of someone else for an admin.
function shifts(): ShiftChoice[] {
  const session = currentSession();
  const own: ShiftChoice = {
    id: "01FAKESHIFT000000000000001",
    business_date: TODAY,
    till: "T1",
    cashier: session?.display_name ?? "",
    opened: `${TODAY} 08:00`,
    closed: true,
  };
  const other: ShiftChoice = { ...own, id: "01FAKESHIFT000000000000002", cashier: "Karim" };
  return session?.permissions.includes("reports_view_all") ? [own, other] : [own];
}

function demoReport(request: ReportRequest): ReportTable {
  switch (request.report) {
    case "daily_summary": {
      const report = table(
        "Daily summary",
        request,
        [
          column("item", "Item", "text"),
          column("count", "Count", "int"),
          column("amount", "Amount", "money"),
        ],
        [
          [text("Gross sales"), count(3), money(45_000)],
          [text("Net sales"), { kind: "empty" }, money(42_500)],
          [text("Total taken"), { kind: "empty" }, money(42_500)],
        ],
      );
      report.sections = [
        table(
          "Payments by method",
          request,
          [column("method", "Method", "text"), column("net", "Net", "money")],
          [[text("Cash"), money(42_500)]],
        ),
      ];
      return report;
    }
    case "shift":
      return table(
        "Z report",
        request,
        [column("item", "Item", "text"), column("amount", "Amount", "money")],
        [
          [text("Net sales"), money(12_000)],
          [text("Expected cash"), money(112_000)],
        ],
      );
    case "stock_value":
    case "low_stock":
    case "near_expiry":
    case "stock_movement":
      return table(
        "Stock",
        request,
        [column("product", "Product", "text"), column("on_hand", "On hand", "qty")],
        [
          [text("Milk 1L"), qty(-2_000)],
          [text("Rice 5kg"), qty(12_500)],
        ],
      );
    default:
      return table(
        "Sales by product",
        request,
        [
          column("name", "Product", "text"),
          column("qty", "Quantity", "qty"),
          column("net", "Net sales", "money"),
          column("share", "Share", "percent"),
        ],
        [
          [text("Bread"), qty(3_000), money(15_000), { kind: "percent", value: 6000 }],
          [text("Milk 1L"), qty(2_000), money(10_000), { kind: "percent", value: 4000 }],
        ],
      );
  }
}

export const reportHandlers = {
  report_choices: (): ReportChoices => {
    requirePermission("reports_view_all");
    const choice = (id: string, name: string): Choice => ({ id, name });
    return {
      today: TODAY,
      tills: [choice("01FAKETILL0000000000000001", "T1")],
      cashiers: [choice("01FAKEUSER0000000000000001", "Shop Owner")],
      categories: [choice("01FAKECATEGORY000000000001", "Dairy")],
      brands: [choice("01FAKEBRAND000000000000001", "Pran")],
      suppliers: [choice("01FAKESUPPLIER000000000001", "Fresh Foods")],
      currency: shop().currency,
      shop_name: shop().name,
    };
  },
  report_customer_search: (args?: Args): Choice[] => {
    requirePermission("reports_view_all");
    const typed = (args?.text as string).toLowerCase();
    const all = [{ id: "01FAKECUSTOMER000000000001", name: "Asha (01711123456)" }];
    return all.filter((each) => each.name.toLowerCase().includes(typed));
  },
  report_shifts: (): ShiftList => {
    if (!currentSession()?.permissions.includes("reports_view_all")) {
      requirePermission("shift_open_close_own");
    }
    return { shifts: shifts(), currency: shop().currency, shop_name: shop().name };
  },
  report_run: (args?: Args): ReportTable => {
    const request = args?.request as ReportRequest;
    mayRun(request);
    if ("from" in request && request.to < request.from) {
      // Rust refuses a range that ends before it starts.
      // eslint-disable-next-line @typescript-eslint/only-throw-error
      throw {
        kind: "validation",
        code: "date_range_backwards",
        params: {},
        field: null,
      } satisfies AppError;
    }
    return demoReport(request);
  },
  report_export_xlsx: (args?: Args): null => {
    mayRun(args?.request as ReportRequest);
    return null;
  },
  report_print: (args?: Args): PrintResult => {
    const request = args?.request as ReportRequest;
    mayRun(request);
    return fakePrint({ kind: "report", request }, testReceiptPicture("mm80"), false);
  },
};
