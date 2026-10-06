import type { AdjustInput } from "../../../generated/AdjustInput";
import type { AppError } from "../../../generated/AppError";
import type { BatchView } from "../../../generated/BatchView";
import type { CountInput } from "../../../generated/CountInput";
import type { CountLineInput } from "../../../generated/CountLineInput";
import type { CountPosted } from "../../../generated/CountPosted";
import type { CountSummary } from "../../../generated/CountSummary";
import type { CountView } from "../../../generated/CountView";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { HistoryQuery } from "../../../generated/HistoryQuery";
import type { HistoryRow } from "../../../generated/HistoryRow";
import type { LowStockView } from "../../../generated/LowStockView";
import type { MismatchView } from "../../../generated/MismatchView";
import type { MoveKind } from "../../../generated/MoveKind";
import type { NearExpiryView } from "../../../generated/NearExpiryView";
import type { Product } from "../../../generated/Product";
import type { Reason } from "../../../generated/Reason";
import type { StockAlerts } from "../../../generated/StockAlerts";
import type { StockFilter } from "../../../generated/StockFilter";
import type { StockList } from "../../../generated/StockList";
import type { StockProduct } from "../../../generated/StockProduct";
import type { StockRow } from "../../../generated/StockRow";
import type { VarianceReport } from "../../../generated/VarianceReport";
import type { VarianceView } from "../../../generated/VarianceView";
import { connectStock, fakeCatalog } from "./catalog";
import { currentSession, recordFakeEvent, requirePermission } from "./people";

// Pretend stock for the fake bridge: levels, batches, history and counts. Rust does the
// real checks (Module - inventory); this stand-in follows the rules the screens and their
// tests need, with the same error codes.

type Args = Record<string, unknown>;

// The fake business day. Batches are "near expiry" within 30 days of it.
const TODAY = "2026-10-05";
const NEAR_EXPIRY_DAYS = 30;

type FakeBatch = {
  id: string;
  product_id: string;
  batch_no: string | null;
  expiry_date: string | null;
  on_hand: number;
  received_at: number;
};

type FakeMove = {
  id: string;
  product_id: string;
  batch_id: string | null;
  qty_delta: number;
  kind: MoveKind;
  reason_code: string | null;
  note: string | null;
  ref_label: string | null;
  unit_cost: number;
  at: number;
  user_name: string | null;
};

type FakeCount = {
  summary: CountSummary;
  // Key "product|batch" → expected at first count, counted so far.
  lines: Map<
    string,
    { product_id: string; batch_id: string | null; expected: number; counted: number }
  >;
};

const reasons: Reason[] = [
  { code: "damage", name: "Damage", direction: "out", is_system: true, active: true },
  { code: "expiry", name: "Expired", direction: "out", is_system: true, active: true },
  { code: "theft", name: "Theft", direction: "out", is_system: true, active: true },
  { code: "count_error", name: "Count error", direction: "both", is_system: true, active: true },
  { code: "opening", name: "Opening stock", direction: "in", is_system: true, active: true },
  { code: "internal_use", name: "Internal use", direction: "out", is_system: true, active: true },
  { code: "other", name: "Other", direction: "both", is_system: true, active: true },
];

let levels = new Map<string, number>();
let batches: FakeBatch[] = [];
let moves: FakeMove[] = [];
let counts: FakeCount[] = [];
let nextId = 1;
let nextCount = 1;
let currency: () => CurrencyFormat = () => {
  throw new Error("The fake stock isn't connected to the fake shop.");
};

// shop.ts calls this once, so the values use the shop's money format.
export function connectStockToShop(shopCurrency: () => CurrencyFormat) {
  currency = shopCurrency;
}

connectStock((id) => levels.get(id) ?? 0);

function fakeId(prefix: string) {
  return prefix + String(nextId++).padStart(26 - prefix.length, "0");
}

function fail(error: AppError): never {
  // Rust's invoke fails with a plain AppError object, so the fake does too.
  // eslint-disable-next-line @typescript-eslint/only-throw-error
  throw error;
}

function refuse(field: string | null, code: string): never {
  return fail({ kind: "validation", field, code, params: {} });
}

function clash(code: string, params: Record<string, string> = {}): never {
  return fail({ kind: "conflict", code, params });
}

function catalogProduct(id: string): Product {
  return (
    fakeCatalog().products.find((each) => each.id === id) ??
    fail({ kind: "not_found", entity: "product", key: id })
  );
}

function unitOf(product: Product) {
  const unit = fakeCatalog().units.find((each) => each.id === product.unit_id);
  return { symbol: unit?.symbol ?? "", decimals: unit?.decimals ?? 0 };
}

function stocked(product: Product) {
  const holds = product.kind === "standard" || product.kind === "variant";
  if (!holds || !product.track_stock) {
    refuse("product_id", "product_keeps_no_stock");
  }
}

function addDays(date: string, days: number) {
  const moment = new Date(`${date}T00:00:00Z`);
  moment.setUTCDate(moment.getUTCDate() + days);
  return moment.toISOString().slice(0, 10);
}

const soon = () => addDays(TODAY, NEAR_EXPIRY_DAYS);

// Writes one history row and moves the level and batch with it, like inventory::record.
function move(entry: Omit<FakeMove, "id" | "at" | "user_name">) {
  moves.push({
    ...entry,
    id: fakeId("01FAKEMOVE"),
    at: Date.now(),
    user_name: currentSession()?.display_name ?? null,
  });
  levels.set(entry.product_id, (levels.get(entry.product_id) ?? 0) + entry.qty_delta);
  const batch = batches.find((each) => each.id === entry.batch_id);
  if (batch) {
    batch.on_hand += entry.qty_delta;
  }
}

// First expiry first: batches with stock by date (undated last), then the rest without a batch.
function fefoSplit(productId: string, out: number): [string | null, number][] {
  const parts: [string | null, number][] = [];
  let left = out;
  const order = batches
    .filter((each) => each.product_id === productId && each.on_hand > 0)
    .sort((a, b) => (a.expiry_date ?? "9999").localeCompare(b.expiry_date ?? "9999"));
  for (const batch of order) {
    if (left === 0) {
      break;
    }
    const take = Math.min(batch.on_hand, left);
    parts.push([batch.id, take]);
    left -= take;
  }
  if (left > 0) {
    parts.push([null, left]);
  }
  return parts;
}

function batchView(batch: FakeBatch): BatchView {
  return {
    id: batch.id,
    batch_no: batch.batch_no,
    expiry_date: batch.expiry_date,
    on_hand: batch.on_hand,
    unit_cost: 0,
    received_at: batch.received_at,
    expired: batch.expiry_date !== null && batch.expiry_date < TODAY,
    near_expiry: batch.expiry_date !== null && batch.expiry_date <= soon(),
  };
}

function stockProduct(product: Product): StockProduct {
  stocked(product);
  const unit = unitOf(product);
  const rule = fakeCatalog().reorder.get(product.id) ?? null;
  return {
    product_id: product.id,
    name: product.name,
    sku: product.sku,
    unit_symbol: unit.symbol,
    unit_decimals: unit.decimals,
    track_expiry: product.track_expiry,
    on_hand: levels.get(product.id) ?? 0,
    reorder_level: rule?.level ?? null,
    reorder_qty: rule?.qty ?? null,
    cost_price: product.cost_price,
    batches: batches
      .filter((each) => each.product_id === product.id && each.on_hand !== 0)
      .sort((a, b) => (a.expiry_date ?? "9999").localeCompare(b.expiry_date ?? "9999"))
      .map(batchView),
  };
}

function stockRow(product: Product): StockRow {
  const unit = unitOf(product);
  const onHand = levels.get(product.id) ?? 0;
  const level = fakeCatalog().reorder.get(product.id)?.level ?? null;
  return {
    product_id: product.id,
    name: product.name,
    sku: product.sku,
    category_name:
      fakeCatalog().categories.find((each) => each.id === product.category_id)?.name ?? "",
    unit_symbol: unit.symbol,
    unit_decimals: unit.decimals,
    on_hand: onHand,
    reorder_level: level,
    cost_price: product.cost_price,
    value: Math.round((product.cost_price * onHand) / 1000),
    low: level !== null && onHand < level,
    negative: onHand < 0,
    expiring: batches.some(
      (each) =>
        each.product_id === product.id &&
        each.on_hand > 0 &&
        each.expiry_date !== null &&
        each.expiry_date <= soon(),
    ),
  };
}

function stockProducts() {
  return fakeCatalog()
    .products.filter(
      (each) =>
        (each.kind === "standard" || each.kind === "variant") &&
        each.track_stock &&
        (each.archived_at === null || (levels.get(each.id) ?? 0) !== 0),
    )
    .sort((a, b) => a.name.localeCompare(b.name));
}

function lowStock(): LowStockView[] {
  return stockProducts()
    .map(stockRow)
    .filter((row) => row.low)
    .map((row) => ({
      product_id: row.product_id,
      name: row.name,
      sku: row.sku,
      unit_symbol: row.unit_symbol,
      unit_decimals: row.unit_decimals,
      on_hand: row.on_hand,
      reorder_level: row.reorder_level ?? 0,
      reorder_qty: fakeCatalog().reorder.get(row.product_id)?.qty ?? 0,
    }));
}

function nearExpiry(): NearExpiryView[] {
  return batches
    .filter((each) => each.on_hand > 0 && each.expiry_date !== null && each.expiry_date <= soon())
    .sort((a, b) => (a.expiry_date ?? "").localeCompare(b.expiry_date ?? ""))
    .map((batch) => {
      const product = catalogProduct(batch.product_id);
      const unit = unitOf(product);
      return {
        batch_id: batch.id,
        product_id: product.id,
        name: product.name,
        unit_symbol: unit.symbol,
        unit_decimals: unit.decimals,
        batch_no: batch.batch_no,
        expiry_date: batch.expiry_date ?? TODAY,
        on_hand: batch.on_hand,
        expired: (batch.expiry_date ?? TODAY) < TODAY,
      };
    });
}

function adjust(input: AdjustInput): number {
  const product = catalogProduct(input.product_id);
  stocked(product);
  if (input.qty_delta === 0) {
    refuse(null, "qty_zero");
  }
  const reason = reasons.find((each) => each.code === input.reason_code);
  if (input.reason_code === "") {
    refuse(null, "reason_needed");
  }
  if (!reason) {
    refuse(null, "reason_unknown");
  }
  const goingIn = input.qty_delta > 0;
  if (reason.direction !== "both" && reason.direction !== (goingIn ? "in" : "out")) {
    refuse(null, "reason_wrong_direction");
  }
  const note = input.note?.trim() ?? "";
  if (reason.code === "other" && note === "") {
    refuse(null, "note_needed");
  }
  const row = {
    product_id: product.id,
    kind: "adjust" as const,
    reason_code: reason.code,
    note: note === "" ? null : note,
    ref_label: null,
    unit_cost: product.cost_price,
  };
  if (input.batch.kind === "new") {
    if (!goingIn) {
      refuse("batch", "new_batch_only_for_stock_in");
    }
    if (product.track_expiry && !input.batch.expiry_date) {
      refuse("expiry_date", "expiry_needed");
    }
    const batch: FakeBatch = {
      id: fakeId("01FAKEBATCH"),
      product_id: product.id,
      batch_no: input.batch.batch_no,
      expiry_date: input.batch.expiry_date,
      on_hand: 0,
      received_at: Date.now(),
    };
    batches.push(batch);
    move({ ...row, batch_id: batch.id, qty_delta: input.qty_delta });
  } else if (input.batch.kind === "existing") {
    const id = input.batch.id;
    const batch =
      batches.find((each) => each.id === id && each.product_id === product.id) ??
      refuse("batch", "batch_of_other_product");
    if (batch.on_hand + input.qty_delta < 0) {
      clash("not_enough_stock_in_batch", { available_milli: String(batch.on_hand) });
    }
    move({ ...row, batch_id: batch.id, qty_delta: input.qty_delta });
  } else if (goingIn) {
    if (product.track_expiry) {
      refuse("expiry_date", "expiry_needed");
    }
    move({ ...row, batch_id: null, qty_delta: input.qty_delta });
  } else {
    for (const [batchId, take] of fefoSplit(product.id, -input.qty_delta)) {
      move({ ...row, batch_id: batchId, qty_delta: -take });
    }
  }
  recordFakeEvent("stock.adjust", currentSession()?.user_id ?? null);
  return levels.get(product.id) ?? 0;
}

function history(query: HistoryQuery): HistoryRow[] {
  const product = catalogProduct(query.product_id);
  let balance = 0;
  const rows: HistoryRow[] = moves
    .filter((each) => each.product_id === product.id)
    .map((each) => {
      balance += each.qty_delta;
      const batch = batches.find((one) => one.id === each.batch_id);
      return {
        id: each.id,
        at: each.at,
        business_date: TODAY,
        kind: each.kind,
        qty_delta: each.qty_delta,
        balance_after: balance,
        batch_no: batch?.batch_no ?? null,
        batch_expiry: batch?.expiry_date ?? null,
        reason_name: reasons.find((one) => one.code === each.reason_code)?.name ?? null,
        note: each.note,
        ref_type: each.ref_label ? "count" : null,
        ref_label: each.ref_label,
        unit_cost: each.unit_cost,
        user_name: each.user_name,
      };
    })
    .filter((row) => query.kinds.length === 0 || query.kinds.includes(row.kind))
    .reverse();
  return rows.slice(query.page.offset, query.page.offset + query.page.limit);
}

function findCount(id: string): FakeCount {
  return (
    counts.find((each) => each.summary.id === id) ??
    fail({ kind: "not_found", entity: "count", key: id })
  );
}

function openCount(id: string): FakeCount {
  const count = findCount(id);
  if (count.summary.status !== "open") {
    clash("count_not_open");
  }
  return count;
}

function countView(count: FakeCount): CountView {
  return {
    count: structuredClone(count.summary),
    lines: [...count.lines.values()].map((line) => {
      const product = catalogProduct(line.product_id);
      const unit = unitOf(product);
      return {
        product_id: product.id,
        name: product.name,
        sku: product.sku,
        unit_symbol: unit.symbol,
        unit_decimals: unit.decimals,
        batch_id: line.batch_id,
        batch_no: batches.find((each) => each.id === line.batch_id)?.batch_no ?? null,
        counted: line.counted,
      };
    }),
  };
}

function variances(count: FakeCount): VarianceView[] {
  const row = (
    productId: string,
    batchId: string | null,
    expected: number,
    counted: number,
    wasCounted: boolean,
  ): VarianceView => {
    const product = catalogProduct(productId);
    const unit = unitOf(product);
    const difference = counted - expected;
    return {
      product_id: product.id,
      name: product.name,
      sku: product.sku,
      unit_symbol: unit.symbol,
      unit_decimals: unit.decimals,
      batch_id: batchId,
      batch_no: batches.find((each) => each.id === batchId)?.batch_no ?? null,
      expected,
      counted,
      difference,
      cost_value: Math.round((product.cost_price * difference) / 1000),
      was_counted: wasCounted,
    };
  };
  const rows = [...count.lines.values()].map((line) =>
    row(line.product_id, line.batch_id, line.expected, line.counted, true),
  );
  if (count.summary.scope === "full" && count.summary.status === "open") {
    const counted = new Set([...count.lines.values()].map((line) => line.product_id));
    for (const [productId, onHand] of levels) {
      if (!counted.has(productId) && onHand !== 0) {
        rows.push(row(productId, null, onHand, 0, false));
      }
    }
  }
  return rows;
}

// Starts the pretend stock again. A set-up shop has demo stock: milk below its
// reorder level in two batches (one expiring soon), bread and soap.
export function resetStock(withDemoStock: boolean) {
  levels = new Map();
  batches = [];
  moves = [];
  counts = [];
  nextId = 1;
  nextCount = 1;
  if (!withDemoStock) {
    return;
  }
  const byName = (name: string) => fakeCatalog().products.find((each) => each.name === name);
  const milk = byName("Milk 1L");
  const bread = byName("Bread");
  const soap = byName("Soap");
  const opening = (
    product: Product | undefined,
    qty: number,
    batchNo?: string,
    expiry?: string,
  ) => {
    if (!product) {
      return;
    }
    let batchId: string | null = null;
    if (batchNo) {
      batchId = fakeId("01FAKEBATCH");
      batches.push({
        id: batchId,
        product_id: product.id,
        batch_no: batchNo,
        expiry_date: expiry ?? null,
        on_hand: 0,
        received_at: Date.now(),
      });
    }
    move({
      product_id: product.id,
      batch_id: batchId,
      qty_delta: qty,
      kind: "opening",
      reason_code: null,
      note: null,
      ref_label: null,
      unit_cost: product.cost_price,
    });
  };
  opening(milk, 10000, "M-0921", "2026-10-12");
  opening(milk, 14000, "M-1001", "2026-11-30");
  opening(bread, 12000);
  opening(soap, 40000);
}

export const stockHandlers = {
  stock_levels: (args?: Args): StockList => {
    requirePermission("stock_view");
    const filter = args?.filter as StockFilter;
    const page = args?.page as { offset: number; limit: number };
    const text = filter.text?.trim().toLowerCase() ?? "";
    const rows = stockProducts()
      .filter(
        (each) =>
          text === "" ||
          each.name.toLowerCase().includes(text) ||
          each.sku.toLowerCase().includes(text),
      )
      .filter((each) => filter.category_id === null || each.category_id === filter.category_id)
      .map(stockRow)
      .filter(
        (row) =>
          filter.show === "all" ||
          (filter.show === "low" && row.low) ||
          (filter.show === "negative" && row.negative) ||
          (filter.show === "expiring" && row.expiring),
      );
    return {
      rows: rows.slice(page.offset, page.offset + page.limit),
      total: rows.length,
      total_value: rows.reduce((sum, row) => sum + row.value, 0),
      currency: currency(),
    };
  },
  stock_product: (args?: Args): StockProduct => {
    requirePermission("stock_view");
    return stockProduct(catalogProduct(String(args?.id)));
  },
  stock_lookup: (args?: Args): StockProduct => {
    requirePermission("stock_view");
    const code = String(args?.code).trim();
    const found = fakeCatalog().products.find(
      (each) =>
        each.archived_at === null &&
        each.kind !== "variant_parent" &&
        (each.barcodes.includes(code) || each.sku === code),
    );
    return stockProduct(found ?? fail({ kind: "not_found", entity: "product", key: code }));
  },
  stock_history: (args?: Args): HistoryRow[] => {
    requirePermission("stock_view");
    return history(args?.query as HistoryQuery);
  },
  stock_reasons: (): Reason[] => {
    requirePermission("stock_view");
    return structuredClone(reasons);
  },
  stock_adjust: (args?: Args): number => {
    requirePermission("stock_adjust");
    return adjust(args?.input as AdjustInput);
  },
  low_stock: (): LowStockView[] => {
    requirePermission("stock_view");
    return lowStock();
  },
  near_expiry: (): NearExpiryView[] => {
    requirePermission("stock_view");
    return nearExpiry();
  },
  stock_alerts: (): StockAlerts => {
    requirePermission("stock_view");
    const low = lowStock();
    const expiring = nearExpiry();
    return {
      low_stock_count: low.length,
      low_stock: low.slice(0, 5),
      near_expiry_count: expiring.length,
      near_expiry: expiring.slice(0, 5),
      negative_count: [...levels.values()].filter((qty) => qty < 0).length,
      near_expiry_days: NEAR_EXPIRY_DAYS,
    };
  },
  stock_verify: (): MismatchView[] => {
    requirePermission("stock_adjust");
    return [];
  },
  count_list: (): CountSummary[] => {
    requirePermission("stock_count_enter");
    return counts.map((each) => structuredClone(each.summary)).reverse();
  },
  count_get: (args?: Args): CountView => {
    requirePermission("stock_count_enter");
    return countView(findCount(String(args?.id)));
  },
  count_create: (args?: Args): string => {
    requirePermission("stock_count_manage");
    const input = args?.input as CountInput;
    if (input.name.trim() === "") {
      refuse(null, "count_name_missing");
    }
    const id = fakeId("01FAKECOUNT");
    const category = fakeCatalog().categories.find((each) => each.id === input.category_id);
    const covers = [category?.name, input.location?.trim()].filter((part) => part).join(" · ");
    counts.push({
      summary: {
        id,
        number: `SC-${String(nextCount++).padStart(6, "0")}`,
        name: input.name.trim(),
        scope: input.scope,
        status: "open",
        covers: input.scope === "full" ? "" : covers,
        created_at: Date.now(),
        created_by_name: currentSession()?.display_name ?? null,
        posted_at: null,
        posted_by_name: null,
      },
      lines: new Map(),
    });
    recordFakeEvent("stock_count.create", currentSession()?.user_id ?? null);
    return id;
  },
  count_line: (args?: Args): CountView => {
    requirePermission("stock_count_enter");
    const count = openCount(String(args?.countId));
    const line = args?.line as CountLineInput;
    stocked(catalogProduct(line.product_id));
    const key = `${line.product_id}|${line.batch_id ?? ""}`;
    const mixes = [...count.lines.values()].some(
      (each) =>
        each.product_id === line.product_id &&
        (each.batch_id === null) !== (line.batch_id === null),
    );
    if (mixes) {
      clash("count_mixes_batch_and_whole");
    }
    const expected =
      line.batch_id === null
        ? (levels.get(line.product_id) ?? 0)
        : (batches.find((each) => each.id === line.batch_id)?.on_hand ?? 0);
    const saved = count.lines.get(key) ?? {
      product_id: line.product_id,
      batch_id: line.batch_id,
      expected,
      counted: 0,
    };
    if (saved.counted + line.qty < 0) {
      refuse(null, "counted_below_zero");
    }
    saved.counted += line.qty;
    count.lines.set(key, saved);
    return countView(count);
  },
  count_variances: (args?: Args): VarianceReport => {
    requirePermission("stock_count_manage");
    const count = findCount(String(args?.id));
    const rows = variances(count);
    return {
      count: structuredClone(count.summary),
      rows,
      total_value: rows.reduce((sum, row) => sum + row.cost_value, 0),
      uncounted: rows.filter((row) => !row.was_counted).length,
      currency: currency(),
    };
  },
  count_post: (args?: Args): CountPosted => {
    requirePermission("stock_count_manage");
    const count = openCount(String(args?.id));
    const rows = variances(count);
    const uncounted = rows.filter((row) => !row.was_counted).length;
    if (uncounted > 0 && args?.confirmUncounted !== true) {
      clash("count_uncounted_needs_confirm", { uncounted: String(uncounted) });
    }
    let written = 0;
    for (const row of rows.filter((each) => each.difference !== 0)) {
      move({
        product_id: row.product_id,
        batch_id: row.batch_id,
        qty_delta: row.difference,
        kind: "count",
        reason_code: null,
        note: null,
        ref_label: count.summary.number,
        unit_cost: catalogProduct(row.product_id).cost_price,
      });
      written += 1;
    }
    count.summary.status = "posted";
    count.summary.posted_at = Date.now();
    count.summary.posted_by_name = currentSession()?.display_name ?? null;
    recordFakeEvent("stock_count.post", currentSession()?.user_id ?? null);
    return {
      rows_written: written,
      backup_file: "/fake/backups/pos-20261005-140000-pre-count-post.sqlite",
    };
  },
  count_cancel: (args?: Args): null => {
    requirePermission("stock_count_manage");
    openCount(String(args?.id)).summary.status = "cancelled";
    recordFakeEvent("stock_count.cancel", currentSession()?.user_id ?? null);
    return null;
  },
} satisfies Record<string, (args?: Args) => unknown>;
