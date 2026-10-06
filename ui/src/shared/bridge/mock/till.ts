import type { AppError } from "../../../generated/AppError";
import type { CartLineView } from "../../../generated/CartLineView";
import type { CartView } from "../../../generated/CartView";
import type { CashMoveInput } from "../../../generated/CashMoveInput";
import type { CheckoutView } from "../../../generated/CheckoutView";
import type { CountLine } from "../../../generated/CountLine";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { PrintResult } from "../../../generated/PrintResult";
import type { HeldCartView } from "../../../generated/HeldCartView";
import type { PaymentEntry } from "../../../generated/PaymentEntry";
import type { PaymentMethodRow } from "../../../generated/PaymentMethodRow";
import type { PaymentMethodSave } from "../../../generated/PaymentMethodSave";
import type { PaymentPreview } from "../../../generated/PaymentPreview";
import type { ResumedCart } from "../../../generated/ResumedCart";
import type { SaleDone } from "../../../generated/SaleDone";
import type { SaleHit } from "../../../generated/SaleHit";
import type { ScaleView } from "../../../generated/ScaleView";
import type { ScanResult } from "../../../generated/ScanResult";
import type { ShiftClosed } from "../../../generated/ShiftClosed";
import type { ShiftState } from "../../../generated/ShiftState";
import type { ShiftView } from "../../../generated/ShiftView";
import type { WeightInput } from "../../../generated/WeightInput";
import { fakeCatalog, searchFakeCatalog } from "./catalog";
import { currentSession, recordFakeEvent, requireOrApproval, requirePermission } from "./people";
import { fakeDrawer, fakePrint, pictures } from "./printer";
import { mockScale } from "./scale";
import { fakeTillPricing } from "./tillPricing";

// A pretend till for the fake bridge: one shift, the cashier's cart, held carts and
// payment methods. Rust does the real work (Module - app); this stand-in prices with
// retail × quantity, 15% tax inside, and rounding to whole units for cash.

type Args = Record<string, unknown>;
type FakeLine = { id: string; product_id: string; qty: number };
type FakeHeld = { id: string; lines: FakeLine[]; note: string | null; at: number; cashier: string };

const DENOMINATIONS = [100000, 50000, 20000, 10000, 5000, 2000, 1000, 500, 200, 100];

let shift: (ShiftView & { cash: number }) | null = null;
let lines: FakeLine[] = [];
let held: FakeHeld[] = [];
let invoices = 0;
// Every sale's invoice number and id, oldest first, for reprints (PAY-10).
let sales: { id: string; invoice_no: string }[] = [];
let nextId = 1;
let methods: PaymentMethodRow[] = [];
let currency: () => CurrencyFormat = () => {
  throw new Error("The fake till isn't connected to the fake shop.");
};

// shop.ts calls this once, so amounts use the shop's money format.
export function connectTill(shopCurrency: () => CurrencyFormat) {
  currency = shopCurrency;
}

// Starts the pretend till again: no shift, an empty cart, cash only.
export function resetTill() {
  shift = null;
  lines = [];
  held = [];
  invoices = 0;
  sales = [];
  nextId = 1;
  methods = [
    method("00000000000000000000000001", "Cash", "cash", true),
    method("01FAKEMETHOD00000000000002", "Card", "card", false, true),
    method("01FAKEMETHOD00000000000003", "bKash", "wallet", false, true),
  ];
}

function method(
  id: string,
  name: string,
  kind: PaymentMethodRow["kind"],
  isSystem: boolean,
  reference = false,
): PaymentMethodRow {
  return {
    id,
    name,
    kind,
    requires_reference: reference,
    active: true,
    sort: 0,
    is_system: isSystem,
  };
}

function fakeId(prefix: string) {
  return prefix + String(nextId++).padStart(26 - prefix.length, "0");
}

function fail(error: AppError): never {
  // Rust's invoke fails with a plain AppError object, so the fake does too.
  // eslint-disable-next-line @typescript-eslint/only-throw-error
  throw error;
}

const refuse = (code: string, params: Record<string, string> = {}): never =>
  fail({ kind: "validation", field: null, code, params });
const clash = (code: string): never => fail({ kind: "conflict", code, params: {} });

function mySelling() {
  if (!shift) {
    return clash("shift_not_open");
  }
  if (shift.user_id !== currentSession()?.user_id) {
    return clash("shift_not_yours");
  }
  return shift;
}

// A weighed product only the till knows, so the weight dialog can be tried (CHK-06).
// It isn't in the pretend catalog, so the catalog screens stay as their tests expect.
const POTATO = {
  id: "01FAKEPOTATO00000000000001",
  name: "Potato",
  sku: "P-POTATO",
  barcodes: ["2000000000015"],
  sale_price: 6000,
  is_weighed: true,
  archived_at: null,
  unit_symbol: "kg",
  unit_decimals: 3,
};

function tillProducts() {
  const { products, units } = fakeCatalog();
  const fromCatalog = products.map((each) => {
    const unit = units.find((one) => one.id === each.unit_id);
    return { ...each, unit_symbol: unit?.symbol ?? "pc", unit_decimals: unit?.decimals ?? 0 };
  });
  return [...fromCatalog, POTATO];
}

function product(id: string) {
  return (
    tillProducts().find((each) => each.id === id) ??
    fail({ kind: "not_found", entity: "product", key: id })
  );
}

function cartView(from: FakeLine[] = lines): CartView {
  const priced: CartLineView[] = from.map((line, index) => {
    const each = product(line.product_id);
    return {
      line_id: line.id,
      seq: index + 1,
      product_id: each.id,
      name: each.name,
      sku: each.sku,
      unit_symbol: each.unit_symbol,
      unit_decimals: each.unit_decimals,
      is_weighed: each.is_weighed,
      qty: line.qty,
      unit_price: each.sale_price,
      base: Math.round((each.sale_price * line.qty) / 1000),
      promotion_name: null,
      discount: 0,
      amount: Math.round((each.sale_price * line.qty) / 1000),
      sticker_price: false,
      price_overridden: false,
      manual_discount: null,
      stock_warning: null,
    };
  });
  return {
    cart_id: "01FAKECART0000000000000001",
    customer: null,
    price_list: null,
    invoice_discount: null,
    lines: priced,
    ...fakeTillPricing(priced),
  };
}

function add(productId: string, qty: number): ScanResult {
  mySelling();
  const each = product(productId);
  if (each.is_weighed) {
    return {
      kind: "needs_weight",
      product: {
        product_id: each.id,
        name: each.name,
        unit_symbol: each.unit_symbol,
        unit_price: each.sale_price,
      },
    };
  }
  const same = lines.find((line) => line.product_id === productId);
  const line = same ?? { id: fakeId("01FAKELINE"), product_id: productId, qty: 0 };
  line.qty += qty;
  if (!same) {
    lines.push(line);
  }
  return { kind: "added", cart: cartView(), line_id: line.id };
}

function scan(input: string): ScanResult {
  requirePermission("checkout_sell");
  const [count, code] = input.includes("*") ? input.split("*", 2) : ["1", input];
  const text = (code ?? "").trim();
  const found = tillProducts().find(
    (each) => each.archived_at === null && (each.barcodes.includes(text) || each.sku === text),
  );
  if (found) {
    return add(found.id, Math.round(Number(count) * 1000));
  }
  const hits = /^\d+$/.test(text) ? [] : saleHits(text);
  return hits.length > 0
    ? { kind: "search", hits }
    : fail({ kind: "not_found", entity: "product_code", key: text });
}

function saleHits(text: string): SaleHit[] {
  return searchFakeCatalog(text, 20).map((hit) => ({
    product_id: hit.product_id,
    name: hit.name,
    sku: hit.sku,
    sale_price: hit.sale_price,
    unit_symbol: hit.unit_symbol,
    on_hand: 25000,
    thumbnail: null,
  }));
}

function preview(payments: PaymentEntry[]): PaymentPreview {
  mySelling();
  const totals = cartView().totals;
  const cashKinds = payments.filter(
    (each) => methods.find((one) => one.id === each.method_id)?.kind === "cash",
  );
  const nonCash = payments
    .filter((each) => !cashKinds.includes(each))
    .reduce((sum, each) => sum + each.amount, 0);
  const rest = Math.max(totals.gross - nonCash, 0);
  const restInCash = Math.round(rest / 100) * 100;
  const due = cashKinds.length > 0 ? nonCash + restInCash : totals.gross;
  if (nonCash > due) {
    refuse("payment_over_remaining", { remaining_minor: String(due) });
  }
  const offered = payments.reduce((sum, each) => sum + each.amount, 0);
  const paid = Math.min(offered, due);
  const quick = DENOMINATIONS.filter((note) => note * 10 >= restInCash)
    .map((note) => Math.ceil(restInCash / note) * note)
    .filter((amount, index, all) => amount > restInCash && all.indexOf(amount) === index)
    .sort((a, b) => a - b)
    .slice(0, 4);
  return {
    gross: totals.gross,
    rounding: due - totals.gross,
    due,
    paid,
    remaining: due - paid,
    change: Math.max(offered - due, 0),
    cash_to_pay: restInCash,
    quick_cash: restInCash > 0 ? quick : [],
    points_offer: null,
    credit_available: null,
  };
}

function complete(payments: PaymentEntry[]): SaleDone {
  requirePermission("checkout_sell");
  if (lines.length === 0) {
    refuse("cart_empty");
  }
  for (const payment of payments) {
    const chosen = methods.find((one) => one.id === payment.method_id);
    if (chosen?.requires_reference && !payment.reference?.trim() && payment.amount > 0) {
      refuse("payment_reference_missing");
    }
  }
  const answer = preview(payments);
  if (answer.remaining > 0) {
    refuse("payment_not_enough", { remaining_minor: String(answer.remaining) });
  }
  invoices += 1;
  if (shift) {
    shift.cash +=
      answer.due -
      payments
        .filter((each) => methods.find((one) => one.id === each.method_id)?.kind !== "cash")
        .reduce((sum, each) => sum + each.amount, 0);
  }
  lines = [];
  const sale = { id: fakeId("01FAKESALE"), invoice_no: `T1-${String(invoices).padStart(6, "0")}` };
  sales.push(sale);
  const cashTaken = answer.change > 0 || answer.cash_to_pay > 0;
  return {
    sale_id: sale.id,
    invoice_no: sale.invoice_no,
    total: answer.due,
    paid: answer.paid,
    change: answer.change,
    cash_taken: cashTaken,
    points_earned: 0,
    cart: cartView(),
    receipt: fakePrint({ kind: "receipt", sale_id: sale.id }, pictures.receipt, cashTaken),
  };
}

// A reprint is marked COPY and never opens the drawer (PAY-10).
function reprint(sale: { id: string } | undefined): PrintResult {
  if (!sale) {
    return clash("no_sale_to_reprint");
  }
  recordFakeEvent("receipt.reprint", currentSession()?.user_id ?? null);
  return fakePrint({ kind: "receipt", sale_id: sale.id }, pictures.receiptCopy, false);
}

function closeShift(counts: CountLine[], forced: boolean): ShiftClosed {
  const open = shift ?? clash("shift_not_open");
  if (!forced && counts.length === 0) {
    refuse("count_missing");
  }
  const counted = counts.reduce((sum, line) => sum + line.denomination * line.count, 0);
  shift = null;
  return {
    shift_id: open.shift_id,
    expected_cash: open.cash,
    counted_cash: counts.length > 0 ? counted : null,
    difference: counts.length > 0 ? counted - open.cash : null,
    forced,
    report: fakePrint({ kind: "shift_report", shift_id: open.shift_id }, pictures.zReport, false),
  };
}

export const tillHandlers = {
  checkout_state: (): CheckoutView => {
    requirePermission("checkout_sell");
    const mine = shift !== null && shift.user_id === currentSession()?.user_id;
    return {
      shift: shift ? { ...shift, mine } : null,
      cart: mine ? cartView() : null,
      held_count: held.length,
      payment_methods: methods.filter((each) => each.active),
      sounds: false,
      currency: currency(),
    };
  },
  checkout_scan: (args?: Args): ScanResult => scan(args?.input as string),
  checkout_search: (args?: Args): SaleHit[] => saleHits(args?.text as string),
  checkout_add_product: (args?: Args): ScanResult => add(String(args?.productId), 1000),
  checkout_read_scale: (): ScaleView => {
    const scale = mockScale();
    return scale.connected
      ? {
          has_scale: true,
          qty: Math.round(Number(scale.weight) * 1000),
          stable: scale.stable,
          problem: null,
        }
      : { has_scale: true, qty: null, stable: false, problem: "timeout" };
  },
  checkout_add_weighed: (args?: Args): CartView => {
    mySelling();
    const weight = args?.weight as WeightInput;
    const qty =
      weight.kind === "manual"
        ? Math.round(Number(weight.text.replace(/\s*k?g$/i, "")) * 1000)
        : Math.round(Number(mockScale().weight) * 1000);
    if (!(qty > 0)) {
      refuse("weight_not_positive");
    }
    lines.push({ id: fakeId("01FAKELINE"), product_id: String(args?.productId), qty });
    return cartView();
  },
  checkout_set_qty: (args?: Args): CartView => {
    const qty = Number(args?.qty);
    const line = lines.find((each) => each.id === args?.lineId);
    if (line && qty > 0 && product(line.product_id).unit_decimals === 0 && qty % 1000 !== 0) {
      refuse("qty_decimals_too_many", { decimals: "0" });
    }
    lines = qty === 0 ? lines.filter((each) => each !== line) : lines;
    if (line && qty > 0) {
      line.qty = qty;
    }
    return cartView();
  },
  checkout_remove_line: (args?: Args): CartView => {
    lines = lines.filter((each) => each.id !== args?.lineId);
    return cartView();
  },
  checkout_hold: (args?: Args): CartView => {
    requirePermission("checkout_hold_resume");
    if (lines.length === 0) {
      refuse("cart_empty");
    }
    held.push({
      id: fakeId("01FAKEHELD"),
      lines,
      note: (args?.note as string | null) ?? null,
      at: Date.now(),
      cashier: currentSession()?.display_name ?? "",
    });
    lines = [];
    return cartView();
  },
  checkout_held_list: (): HeldCartView[] =>
    held.map((each) => {
      const totals = cartView(each.lines).totals;
      return {
        cart_id: each.id,
        held_at: each.at,
        cashier_name: each.cashier,
        item_count: totals.item_count,
        total: totals.total,
        note: each.note,
      };
    }),
  checkout_resume: (args?: Args): ResumedCart => {
    if (lines.length > 0) {
      clash("cart_in_progress");
    }
    const found = held.find((each) => each.id === args?.cartId) ?? clash("cart_not_held");
    held = held.filter((each) => each !== found);
    lines = found.lines;
    return { cart: cartView(), price_changed: [] };
  },
  checkout_cancel: (): CartView => {
    requirePermission("checkout_cancel_cart");
    if (lines.length > 0) {
      recordFakeEvent("cart.cancel", currentSession()?.user_id ?? null);
    }
    lines = [];
    return cartView();
  },
  checkout_payment_preview: (args?: Args): PaymentPreview =>
    preview((args?.payments as PaymentEntry[]).filter((each) => each.amount !== 0)),
  checkout_complete: (args?: Args): SaleDone =>
    complete((args?.payments as PaymentEntry[]).filter((each) => each.amount !== 0)),
  shift_current: (): ShiftState => ({
    shift: shift ? { ...shift, mine: shift.user_id === currentSession()?.user_id } : null,
    denominations: DENOMINATIONS,
    blind_close: true,
    expected_cash: null,
    currency: currency(),
  }),
  shift_count_total: (args?: Args): number =>
    (args?.counts as CountLine[]).reduce((sum, line) => sum + line.denomination * line.count, 0),
  shift_open: (args?: Args): ShiftView => {
    requirePermission("shift_open_close_own");
    if (shift) {
      clash("shift_already_open");
    }
    const counts = args?.counts as CountLine[];
    const float =
      counts.length > 0
        ? counts.reduce((sum, line) => sum + line.denomination * line.count, 0)
        : Number(args?.float);
    const session = currentSession();
    shift = {
      shift_id: fakeId("01FAKESHIFT"),
      user_id: session?.user_id ?? "",
      cashier_name: session?.display_name ?? "",
      business_date: "2026-10-05",
      opened_at: Date.now(),
      opening_float: float,
      mine: true,
      cash: float,
    };
    recordFakeEvent("shift.open", currentSession()?.user_id ?? null);
    return shift;
  },
  shift_cash_move: (args?: Args): PrintResult => {
    requireOrApproval("shift_cash_in_out", args?.approval);
    const input = args?.input as CashMoveInput;
    if (input.reason.trim() === "") {
      refuse("reason_missing");
    }
    if (shift) {
      shift.cash += input.kind === "cash_in" ? input.amount : -input.amount;
    }
    recordFakeEvent("shift.cash_move", currentSession()?.user_id ?? null);
    return fakeDrawer();
  },
  shift_no_sale: (args?: Args): PrintResult => {
    requireOrApproval("shift_no_sale_drawer", args?.approval);
    recordFakeEvent("drawer.no_sale", currentSession()?.user_id ?? null);
    return fakeDrawer();
  },
  shift_x_report: (): PrintResult => {
    requirePermission("shift_open_close_own");
    const open = mySelling();
    return fakePrint({ kind: "shift_report", shift_id: open.shift_id }, pictures.xReport, false);
  },
  receipt_reprint_last: (): PrintResult => {
    requirePermission("receipt_reprint_last");
    return reprint(sales.at(-1));
  },
  receipt_reprint: (args?: Args): PrintResult => {
    requirePermission("receipt_reprint_any");
    const invoice = String(args?.invoiceNo).trim();
    const sale =
      sales.find((each) => each.invoice_no === invoice) ??
      fail({ kind: "not_found", entity: "sale", key: invoice });
    return reprint(sale);
  },
  shift_close: (args?: Args): ShiftClosed => {
    requirePermission("shift_open_close_own");
    if (lines.length > 0) {
      clash("shift_has_open_carts");
    }
    return closeShift(args?.counts as CountLine[], false);
  },
  shift_force_close: (args?: Args): ShiftClosed => {
    requireOrApproval("shift_force_close", args?.approval);
    if (lines.length > 0) {
      held.push({
        id: fakeId("01FAKEHELD"),
        lines,
        note: null,
        at: Date.now(),
        cashier: shift?.cashier_name ?? "",
      });
      lines = [];
    }
    return closeShift(args?.counts as CountLine[], true);
  },
  payment_method_list: (): PaymentMethodRow[] => {
    requirePermission("settings_manage");
    return structuredClone(methods);
  },
  payment_method_save: (args?: Args): string => {
    requirePermission("settings_manage");
    const input = args?.input as PaymentMethodSave;
    if (input.name.trim() === "") {
      refuse("payment_method_name_missing");
    }
    const id = input.id ?? fakeId("01FAKEMETHOD");
    const row = { ...input, id, is_system: false };
    methods = input.id
      ? methods.map((each) => (each.id === id ? { ...row, is_system: each.is_system } : each))
      : [...methods, row];
    return id;
  },
} satisfies Record<string, (args?: Args) => unknown>;
