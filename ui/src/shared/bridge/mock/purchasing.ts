import type { AgingView } from "../../../generated/AgingView";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { Grn } from "../../../generated/Grn";
import type { GrnDone } from "../../../generated/GrnDone";
import type { GrnInput } from "../../../generated/GrnInput";
import type { GrnLine } from "../../../generated/GrnLine";
import type { GrnPreview } from "../../../generated/GrnPreview";
import type { GrnRow } from "../../../generated/GrnRow";
import type { GrnStart } from "../../../generated/GrnStart";
import type { LedgerKind } from "../../../generated/LedgerKind";
import type { NewOrderLine } from "../../../generated/NewOrderLine";
import type { Order } from "../../../generated/Order";
import type { OrderList } from "../../../generated/OrderList";
import type { OrderSave } from "../../../generated/OrderSave";
import type { OrderStatus } from "../../../generated/OrderStatus";
import type { OrderView } from "../../../generated/OrderView";
import type { ReturnRow } from "../../../generated/ReturnRow";
import type { Supplier } from "../../../generated/Supplier";
import type { SupplierList } from "../../../generated/SupplierList";
import type { SupplierPage } from "../../../generated/SupplierPage";
import type { SupplierPaymentDone } from "../../../generated/SupplierPaymentDone";
import type { SupplierPaymentInput } from "../../../generated/SupplierPaymentInput";
import type { SupplierReturnInput } from "../../../generated/SupplierReturnInput";
import type { SupplierReturnRef } from "../../../generated/SupplierReturnRef";
import type { SupplierSave } from "../../../generated/SupplierSave";
import type { SupplierStatement } from "../../../generated/SupplierStatement";
import { fakeCatalog } from "./catalog";
import { currentSession, recordFakeEvent, requirePermission } from "./people";
import { fakeDrawer } from "./printer";

// A pretend purchasing office for the fake bridge: suppliers, purchase orders,
// deliveries, supplier returns and what the shop owes. Rust does the real work
// (Module - app); here a delivery's new cost is simply the cost received.

type Args = Record<string, unknown>;
type LedgerRow = {
  id: string;
  supplier_id: string;
  kind: LedgerKind;
  amount: number;
  reference: string | null;
  at: number;
  date: string;
};

let suppliers: Supplier[] = [];
let orders: Order[] = [];
let grns: Grn[] = [];
let returns: ReturnRow[] = [];
let ledger: LedgerRow[] = [];
let nextId = 1;
let numbers = { po: 0, grn: 0, sr: 0 };
let currency: () => CurrencyFormat = () => {
  throw new Error("The fake purchasing office isn't connected to the fake shop.");
};

// shop.ts calls this once, so amounts use the shop's money format.
export function connectPurchasingToShop(shopCurrency: () => CurrencyFormat) {
  currency = shopCurrency;
}

// Starts the pretend purchasing office again. A set-up shop has one supplier.
export function resetPurchasing(withDemoSupplier: boolean) {
  suppliers = [];
  orders = [];
  grns = [];
  returns = [];
  ledger = [];
  nextId = 1;
  numbers = { po: 0, grn: 0, sr: 0 };
  if (withDemoSupplier) {
    suppliers.push(newSupplier("Rahim Traders", 30));
  }
}

function fakeId(prefix: string) {
  return prefix + String(nextId++).padStart(26 - prefix.length, "0");
}

function number(prefix: string, value: number) {
  return `${prefix}-${String(value).padStart(6, "0")}`;
}

const today = () => new Date().toISOString().slice(0, 10);

function fail(error: unknown): never {
  // Rust's invoke fails with a plain AppError object, so the fake does too.
  throw error;
}

const refuse = (field: string | null, code: string): never =>
  fail({ kind: "validation", field, code, params: {} });

function newSupplier(name: string, terms: number): Supplier {
  return {
    id: fakeId("01FAKESUPPLIER"),
    name,
    contact_person: null,
    phone: null,
    email: null,
    address: null,
    tax_number: null,
    payment_terms_days: terms,
    notes: null,
    archived_at: null,
    created_at: Date.now(),
    updated_at: Date.now(),
  };
}

function supplier(id: unknown): Supplier {
  return (
    suppliers.find((each) => each.id === id) ??
    fail({ kind: "not_found", entity: "supplier", key: String(id) })
  );
}

function order(id: unknown): Order {
  return (
    orders.find((each) => each.id === id) ??
    fail({ kind: "not_found", entity: "order", key: String(id) })
  );
}

function product(id: string) {
  return (
    fakeCatalog().products.find((each) => each.id === id) ??
    fail({ kind: "not_found", entity: "product", key: id })
  );
}

const value = (cost: number, qty: number) => Math.round((cost * qty) / 1000);

function balance(supplierId: string): number {
  return ledger
    .filter((row) => row.supplier_id === supplierId)
    .reduce((sum, row) => sum + row.amount, 0);
}

function addLedger(supplierId: string, kind: LedgerKind, amount: number, reference: string | null) {
  ledger.push({
    id: fakeId("01FAKELEDGER"),
    supplier_id: supplierId,
    kind,
    amount,
    reference,
    at: Date.now(),
    date: today(),
  });
}

function orderView(found: Order): OrderView {
  const values = found.lines.map((line) => value(line.unit_cost, line.qty));
  return {
    order: structuredClone(found),
    supplier_name: supplier(found.supplier_id).name,
    line_values: values,
    total: values.reduce((sum, each) => sum + each, 0),
    currency: currency(),
  };
}

function grnRow(found: Grn): GrnRow {
  return found.row;
}

function preview(input: GrnInput): GrnPreview {
  const lines = input.lines.map((line) => {
    const each = product(line.product_id);
    return {
      product_id: each.id,
      name: each.name,
      value: value(line.unit_cost, line.qty),
      tracks_expiry: each.track_expiry,
      old_cost: each.cost_price,
      new_cost: line.unit_cost,
    };
  });
  const linesTotal = lines.reduce((sum, line) => sum + line.value, 0);
  return {
    lines,
    lines_total: linesTotal,
    invoice_total: input.invoice_total,
    difference: input.invoice_total - linesTotal,
  };
}

function saveGrn(input: GrnInput): GrnDone {
  const from = supplier(input.supplier_id);
  if (from.archived_at !== null) {
    fail({ kind: "conflict", code: "supplier_archived", params: {} });
  }
  if (input.lines.length === 0) {
    refuse("lines", "lines_missing");
  }
  const checked = preview(input);
  input.lines.forEach((line, index) => {
    if (line.qty <= 0) {
      refuse("qty", "qty_not_positive");
    }
    if (checked.lines[index]?.tracks_expiry && line.expiry_date === null) {
      refuse("expiry_date", "expiry_needed");
    }
  });
  numbers.grn += 1;
  const id = fakeId("01FAKEGRN");
  const lines: GrnLine[] = input.lines.map((line) => ({
    id: fakeId("01FAKEGRNLINE"),
    order_line_id: line.order_line_id,
    product_id: line.product_id,
    product_name: product(line.product_id).name,
    qty: line.qty,
    unit_cost: line.unit_cost,
    batch_no: line.batch_no,
    expiry_date: line.expiry_date,
    inventory_batch_id: line.expiry_date || line.batch_no ? fakeId("01FAKEBATCH") : null,
  }));
  let status: OrderStatus | null = null;
  const over: string[] = [];
  let orderNumber: string | null = null;
  if (input.order_id) {
    const filled = order(input.order_id);
    orderNumber = filled.number;
    for (const line of input.lines) {
      const ordered = filled.lines.find((each) => each.id === line.order_line_id);
      if (ordered) {
        ordered.received += line.qty;
        if (ordered.received > ordered.qty) {
          over.push(ordered.product_name);
        }
      }
    }
    filled.status = filled.lines.every((each) => each.received >= each.qty)
      ? "received"
      : "partial";
    status = filled.status;
  }
  grns.push({
    row: {
      id,
      number: number("GRN", numbers.grn),
      supplier_id: from.id,
      supplier_name: from.name,
      order_number: orderNumber,
      supplier_invoice_no: input.supplier_invoice_no,
      invoice_total: input.invoice_total,
      lines_total: checked.lines_total,
      received_at: Date.now(),
      business_date: today(),
    },
    notes: input.notes,
    lines,
  });
  addLedger(from.id, "invoice", input.invoice_total, input.supplier_invoice_no);
  recordFakeEvent("grn.save", currentSession()?.user_id ?? null);
  return {
    grn_id: id,
    number: number("GRN", numbers.grn),
    lines_total: checked.lines_total,
    invoice_total: input.invoice_total,
    total_differs: checked.difference !== 0,
    over_received: over,
    order_status: status,
    cost_changes: checked.lines
      .filter((line) => line.new_cost !== line.old_cost)
      .map((line) => ({
        product_id: line.product_id,
        name: line.name,
        old_cost: line.old_cost,
        new_cost: line.new_cost,
      })),
  };
}

function manage() {
  requirePermission("purchasing_manage");
}

export const purchasingHandlers = {
  supplier_list: (args?: Args): SupplierList => {
    manage();
    return {
      rows: suppliers
        .filter((each) => args?.includeArchived === true || each.archived_at === null)
        .sort((a, b) => a.name.localeCompare(b.name))
        .map((each) => ({ supplier: structuredClone(each), balance: balance(each.id) })),
      currency: currency(),
    };
  },
  supplier_get: (args?: Args): SupplierPage => {
    manage();
    const found = supplier(args?.id);
    return {
      supplier: structuredClone(found),
      balance: balance(found.id),
      grns: grns
        .filter((grn) => grn.row.supplier_id === found.id)
        .map(grnRow)
        .reverse(),
      payment_methods: [
        { id: "00000000000000000000000001", name: "Cash", kind: "cash", requires_reference: false },
        {
          id: "01FAKEMETHOD00000000000002",
          name: "Card",
          kind: "card",
          requires_reference: false,
        },
      ],
      has_open_shift: false,
      currency: currency(),
    };
  },
  supplier_save: (args?: Args): string => {
    manage();
    const save = args?.input as SupplierSave;
    if (save.input.name.trim() === "") {
      refuse("name", "supplier_name_missing");
    }
    recordFakeEvent("supplier.change", currentSession()?.user_id ?? null);
    if (save.id) {
      const found = supplier(save.id);
      Object.assign(found, save.input, { name: save.input.name.trim(), updated_at: Date.now() });
      return found.id;
    }
    const added = { ...newSupplier(save.input.name.trim(), 0), ...save.input };
    added.name = save.input.name.trim();
    suppliers.push(added);
    return added.id;
  },
  supplier_archive: (args?: Args): null => {
    manage();
    supplier(args?.id).archived_at = Date.now();
    return null;
  },
  po_list: (args?: Args): OrderList => {
    manage();
    const status = (args?.status as OrderStatus | null) ?? null;
    return {
      rows: [...orders]
        .reverse()
        .filter((each) => status === null || each.status === status)
        .map((each) => {
          const view = orderView(each);
          return {
            id: each.id,
            number: each.number,
            supplier_id: each.supplier_id,
            supplier_name: view.supplier_name,
            status: each.status,
            expected_date: each.expected_date,
            created_at: each.created_at,
            line_count: each.lines.length,
            total: view.total,
          };
        }),
      currency: currency(),
    };
  },
  po_get: (args?: Args): OrderView => {
    manage();
    return orderView(order(args?.id));
  },
  po_save: (args?: Args): string => {
    manage();
    const save = args?.input as OrderSave;
    const from = supplier(save.supplier_id);
    if (from.archived_at !== null) {
      fail({ kind: "conflict", code: "supplier_archived", params: {} });
    }
    if (save.lines.length === 0) {
      refuse("lines", "lines_missing");
    }
    const lines = save.lines.map((line) => {
      if (line.qty <= 0) {
        refuse("qty", "qty_not_positive");
      }
      const each = product(line.product_id);
      return {
        id: fakeId("01FAKEORDERLINE"),
        product_id: each.id,
        product_name: each.name,
        sku: each.sku,
        qty: line.qty,
        unit_cost: line.unit_cost,
        received: 0,
      };
    });
    recordFakeEvent("purchase_order.change", currentSession()?.user_id ?? null);
    if (save.id) {
      const found = order(save.id);
      if (found.status !== "draft") {
        fail({ kind: "conflict", code: "order_not_draft", params: {} });
      }
      Object.assign(found, {
        supplier_id: from.id,
        expected_date: save.expected_date,
        notes: save.notes,
        lines,
      });
      return found.id;
    }
    numbers.po += 1;
    const id = fakeId("01FAKEORDER");
    orders.push({
      id,
      number: number("PO", numbers.po),
      supplier_id: from.id,
      status: "draft",
      expected_date: save.expected_date,
      notes: save.notes,
      created_at: Date.now(),
      created_by: currentSession()?.user_id ?? "",
      sent_at: null,
      lines,
    });
    return id;
  },
  po_suggest: (args?: Args): NewOrderLine[] => {
    manage();
    const { products, reorder } = fakeCatalog();
    return products
      .filter((each) => each.preferred_supplier_id === args?.supplierId && reorder.has(each.id))
      .map((each) => ({
        product_id: each.id,
        product_name: each.name,
        sku: each.sku,
        qty: reorder.get(each.id)?.qty ?? 0,
        unit_cost: each.cost_price,
      }));
  },
  po_mark_sent: (args?: Args): null => {
    manage();
    const found = order(args?.id);
    if (found.status === "draft") {
      found.status = "sent";
      found.sent_at = Date.now();
    }
    return null;
  },
  po_export: (args?: Args): null => {
    manage();
    const found = order(args?.id);
    if (found.status === "draft") {
      found.status = "sent";
      found.sent_at = Date.now();
    }
    return null;
  },
  po_cancel: (args?: Args): null => {
    manage();
    const found = order(args?.id);
    if (found.lines.some((line) => line.received > 0)) {
      fail({ kind: "conflict", code: "order_has_receipts", params: {} });
    }
    found.status = "cancelled";
    return null;
  },
  grn_start: (args?: Args): GrnStart => {
    manage();
    const found = order(args?.orderId);
    return {
      supplier_id: found.supplier_id,
      supplier_name: supplier(found.supplier_id).name,
      order_id: found.id,
      order_number: found.number,
      lines: found.lines.map((line) => ({
        order_line_id: line.id,
        product_id: line.product_id,
        name: line.product_name,
        sku: line.sku,
        ordered: line.qty,
        received: line.received,
        remaining: Math.max(line.qty - line.received, 0),
        unit_cost: line.unit_cost,
        tracks_expiry: product(line.product_id).track_expiry,
      })),
      currency: currency(),
    };
  },
  grn_preview: (args?: Args): GrnPreview => {
    manage();
    return preview(args?.input as GrnInput);
  },
  grn_save: (args?: Args): GrnDone => {
    manage();
    return saveGrn(args?.input as GrnInput);
  },
  grn_list: (): GrnRow[] => {
    manage();
    return grns.map(grnRow).reverse();
  },
  grn_get: (args?: Args): Grn => {
    manage();
    return structuredClone(
      grns.find((each) => each.row.id === args?.id) ??
        fail({ kind: "not_found", entity: "grn", key: String(args?.id) }),
    );
  },
  supplier_return_save: (args?: Args): SupplierReturnRef => {
    manage();
    const input = args?.input as SupplierReturnInput;
    const to = supplier(input.supplier_id);
    if (input.lines.length === 0) {
      refuse("lines", "lines_missing");
    }
    const total = input.lines.reduce(
      (sum, line) => sum + value(line.unit_cost ?? product(line.product_id).cost_price, line.qty),
      0,
    );
    numbers.sr += 1;
    const id = fakeId("01FAKESRETURN");
    returns.unshift({
      id,
      number: number("SR", numbers.sr),
      supplier_id: to.id,
      supplier_name: to.name,
      grn_number: grns.find((each) => each.row.id === input.grn_id)?.row.number ?? null,
      total,
      reason: input.reason,
      at: Date.now(),
      line_count: input.lines.length,
    });
    addLedger(to.id, "return_credit", -total, number("SR", numbers.sr));
    recordFakeEvent("supplier_return.save", currentSession()?.user_id ?? null);
    return { id, number: number("SR", numbers.sr), total };
  },
  supplier_return_list: (): ReturnRow[] => {
    manage();
    return structuredClone(returns);
  },
  supplier_payment: (args?: Args): SupplierPaymentDone => {
    requirePermission("purchasing_pay");
    const input = args?.input as SupplierPaymentInput;
    if (input.amount <= 0) {
      refuse("amount", "amount_not_positive");
    }
    if (input.from_drawer) {
      fail({ kind: "conflict", code: "shift_not_open", params: {} });
    }
    addLedger(supplier(input.supplier_id).id, "payment", -input.amount, input.reference);
    recordFakeEvent("supplier.payment", currentSession()?.user_id ?? null);
    return { balance: balance(input.supplier_id), drawer: input.from_drawer ? fakeDrawer() : null };
  },
  payables_aging: (): AgingView => {
    manage();
    return {
      rows: suppliers
        .filter((each) => balance(each.id) !== 0)
        .map((each) => ({
          supplier_id: each.id,
          name: each.name,
          current: balance(each.id),
          days_1_30: 0,
          days_31_60: 0,
          days_61_90: 0,
          days_over_90: 0,
          balance: balance(each.id),
        })),
      today: today(),
      currency: currency(),
    };
  },
  supplier_statement: (args?: Args): SupplierStatement => {
    manage();
    const found = supplier(args?.id);
    const from = String(args?.from);
    const to = String(args?.to);
    const theirs = ledger.filter((row) => row.supplier_id === found.id);
    const opening = theirs
      .filter((row) => row.date < from)
      .reduce((sum, row) => sum + row.amount, 0);
    let running = opening;
    const lines = theirs
      .filter((row) => row.date >= from && row.date <= to)
      .map((row) => {
        running += row.amount;
        return {
          id: row.id,
          at: row.at,
          business_date: row.date,
          kind: row.kind,
          amount: row.amount,
          method_kind: null,
          reference: row.reference,
          ref_type: null,
          ref_id: null,
          due_date: null,
          balance_after: running,
        };
      });
    return {
      supplier_id: found.id,
      supplier_name: found.name,
      from,
      to,
      opening,
      lines,
      closing: running,
    };
  },
  supplier_statement_export: (args?: Args): number => {
    manage();
    return purchasingHandlers.supplier_statement(args).lines.length;
  },
} satisfies Record<string, (args?: Args) => unknown>;
