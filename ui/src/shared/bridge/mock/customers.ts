import type { AppError } from "../../../generated/AppError";
import type { CartCustomer } from "../../../generated/CartCustomer";
import type { Customer } from "../../../generated/Customer";
import type { CustomerPage } from "../../../generated/CustomerPage";
import type { CustomerPaymentDone } from "../../../generated/CustomerPaymentDone";
import type { CustomerPaymentInput } from "../../../generated/CustomerPaymentInput";
import type { CustomerRow } from "../../../generated/CustomerRow";
import type { CustomerSave } from "../../../generated/CustomerSave";
import type { PointsRow } from "../../../generated/PointsRow";
import type { PriceList } from "../../../generated/PriceList";
import type { PriceListItem } from "../../../generated/PriceListItem";
import type { PriceListItemRow } from "../../../generated/PriceListItemRow";
import type { PriceListSave } from "../../../generated/PriceListSave";
import type { Promotion } from "../../../generated/Promotion";
import type { PromotionSave } from "../../../generated/PromotionSave";
import type { PurchaseRow } from "../../../generated/PurchaseRow";
import type { Statement } from "../../../generated/Statement";
import type { StatementLine } from "../../../generated/StatementLine";
import type { Tier } from "../../../generated/Tier";
import type { TierSave } from "../../../generated/TierSave";
import { fakeCatalog } from "./catalog";
import { currentSession, recordFakeEvent, requirePermission } from "./people";

// Pretend price lists, tiers, promotions and customers with their points and credit
// (C-M8). Rust does the real work in `pricing`, `customers` and `app`; this only feeds
// the screens in `pnpm dev:mock` and tests.

type Args = Record<string, unknown>;
type FakeCustomer = {
  customer: Customer;
  points: PointsRow[];
  credit: StatementLine[];
  purchases: PurchaseRow[];
};

let priceLists: PriceList[] = [];
let listItems: Record<string, PriceListItem[]> = {};
let tiers: Tier[] = [];
let promotions: Promotion[] = [];
let customers: FakeCustomer[] = [];
let nextId = 100;
let tillCash: (amount: number) => boolean = () => false;

function fakeId(prefix: string) {
  return prefix + String(nextId++).padStart(26 - prefix.length, "0");
}

function fail(error: AppError): never {
  // Rust's invoke fails with a plain AppError object, so the fake does too.
  // eslint-disable-next-line @typescript-eslint/only-throw-error
  throw error;
}

const refuse = (field: string | null, code: string, params: Record<string, string> = {}): never =>
  fail({ kind: "validation", field, code, params });
const clash = (code: string, params: Record<string, string> = {}): never =>
  fail({ kind: "conflict", code, params });
const missing = (entity: string, key: string): never => fail({ kind: "not_found", entity, key });

// till.ts tells this file how to put cash in the open shift's drawer (CUS-05).
export function connectCustomersToTill(cashIn: (amount: number) => boolean) {
  tillCash = cashIn;
}

// Starts again. A set-up shop has a Wholesale list, a Gold tier and two customers.
export function resetCustomers(setUp: boolean) {
  priceLists = [];
  listItems = {};
  tiers = [];
  promotions = [];
  customers = [];
  // New ids start above the demo rows' fixed ids, so they never clash.
  nextId = 100;
  if (!setUp) {
    return;
  }
  priceLists = [
    { id: "01FAKELIST0000000000000001", name: "Wholesale", active: true, created_at: 0 },
  ];
  tiers = [
    {
      id: "01FAKETIER0000000000000001",
      name: "Gold",
      price_list_id: null,
      discount: 500,
      sort: 0,
      archived_at: null,
    },
  ];
  addFake("01FAKECUST0000000000000001", "+8801711123456", "Asha Begum", tiers[0]?.id ?? null);
  addFake("01FAKECUST0000000000000002", "+8801811000222", "Babul Mia", null);
  const asha = customers[0];
  if (asha) {
    asha.points.push({ at: 0, kind: "earn", points: 120, reason: null });
    asha.customer.credit_limit = 500000;
  }
}

function addFake(id: string, phone: string, name: string, tierId: string | null) {
  customers.push({
    customer: {
      id,
      phone,
      name,
      email: null,
      address: null,
      tier_id: tierId,
      credit_limit: 0,
      loyalty_opt_out: false,
      notes: null,
      archived_at: null,
      created_at: 0,
      updated_at: 0,
    },
    points: [],
    credit: [],
    purchases: [],
  });
}

const pointsOf = (each: FakeCustomer) => each.points.reduce((sum, row) => sum + row.points, 0);
const creditOf = (each: FakeCustomer) => each.credit.at(-1)?.balance_after ?? 0;

function tierName(tierId: string | null): string | null {
  const tier = tiers.find((each) => each.id === tierId);
  return tier && tier.archived_at === null ? tier.name : null;
}

function found(id: unknown): FakeCustomer {
  return customers.find((each) => each.customer.id === id) ?? missing("customer", String(id));
}

function row(each: FakeCustomer): CustomerRow {
  return {
    id: each.customer.id,
    phone: each.customer.phone,
    name: each.customer.name,
    tier_name: tierName(each.customer.tier_id),
    points: pointsOf(each),
    credit: creditOf(each),
    archived: each.customer.archived_at !== null,
  };
}

// Bangladesh numbers as Rust's `normalize_phone` cleans them: "01711…" → "+8801711…".
function cleanPhone(typed: string): string {
  const digits = typed.replace(/[\s-]/g, "");
  const local = digits.startsWith("+880") ? digits.slice(4) : digits.replace(/^0/, "");
  if (!/^1\d{9}$/.test(local)) {
    refuse("phone", "phone_invalid");
  }
  return `+880${local}`;
}

function matches(each: FakeCustomer, typed: string): boolean {
  const text = typed.trim().toLowerCase();
  const digits = text.replace(/\D/g, "").replace(/^0/, "");
  return (
    text === "" ||
    each.customer.name.toLowerCase().includes(text) ||
    (digits.length > 0 && each.customer.phone.includes(digits))
  );
}

// ---- What the fake till needs -------------------------------------------------------

// The customer as the till shows them, or null when the id is unknown.
export function fakeCartCustomer(id: string | null): CartCustomer | null {
  const each = customers.find((one) => one.customer.id === id);
  if (!each) {
    return null;
  }
  return {
    id: each.customer.id,
    name: each.customer.name,
    phone: each.customer.phone,
    tier_name: tierName(each.customer.tier_id),
    points: pointsOf(each),
    credit: creditOf(each),
    credit_limit: each.customer.credit_limit,
  };
}

// Refuses archived and unknown customers, as `checkout_set_customer` does.
export function fakeAttachable(id: string) {
  if (found(id).customer.archived_at !== null) {
    refuse(null, "customer_archived");
  }
}

// The tier discount (in hundredths of a percent) the customer gets, 0 without a tier.
export function fakeTierDiscount(id: string | null): number {
  const each = customers.find((one) => one.customer.id === id);
  const tier = tiers.find((one) => one.id === each?.customer.tier_id);
  return tier && tier.archived_at === null ? tier.discount : 0;
}

// The price of a product in a price list, or null when the list doesn't have it.
export function fakeListPrice(listId: string | null, productId: string): number | null {
  if (listId === null) {
    return null;
  }
  return listItems[listId]?.find((each) => each.product_id === productId)?.price ?? null;
}

export function fakeActiveLists() {
  return priceLists.filter((each) => each.active).map((each) => ({ id: each.id, name: each.name }));
}

export function fakeListChoice(id: string | null) {
  const list = priceLists.find((each) => each.id === id);
  return list ? { id: list.id, name: list.name } : null;
}

export function fakeCheckList(id: string) {
  const list = priceLists.find((each) => each.id === id) ?? missing("price_list", id);
  if (!list.active) {
    refuse(null, "price_list_inactive");
  }
}

// A completed sale for the customer: points used and earned, credit, and history.
export function fakeCustomerSale(
  id: string,
  sale: { id: string; invoice_no: string; total: number },
  usedPoints: number,
  onCredit: number,
  earned: number,
) {
  const each = found(id);
  const now = Date.now();
  if (usedPoints > 0) {
    each.points.push({ at: now, kind: "redeem", points: -usedPoints, reason: null });
  }
  if (earned > 0 && !each.customer.loyalty_opt_out) {
    each.points.push({ at: now, kind: "earn", points: earned, reason: null });
  }
  if (onCredit > 0) {
    addCredit(each, onCredit, "credit_sale", null, { type: "sale", id: sale.id });
  }
  each.purchases.unshift({
    kind: "sale",
    id: sale.id,
    number: sale.invoice_no,
    at: now,
    total: sale.total,
    voided: false,
  });
}

function addCredit(
  each: FakeCustomer,
  amount: number,
  kind: StatementLine["kind"],
  methodKind: string | null,
  ref: { type: string; id: string } | null,
) {
  each.credit.push({
    id: fakeId("01FAKECREDIT"),
    at: Date.now(),
    business_date: "2026-10-05",
    kind,
    amount,
    method_kind: methodKind,
    reference: null,
    ref_type: ref?.type ?? null,
    ref_id: ref?.id ?? null,
    balance_after: creditOf(each) + amount,
  });
}

// ---- The commands ------------------------------------------------------------------

export const customerHandlers = {
  price_list_list: (): PriceList[] => {
    requirePermission("pricing_manage");
    return [...priceLists].sort((a, b) => a.name.localeCompare(b.name));
  },
  price_list_save: (args?: Args): string => {
    requirePermission("pricing_manage");
    const input = args?.input as PriceListSave;
    const name = input.name.trim();
    if (name === "") {
      refuse("name", "price_list_name_missing");
    }
    if (priceLists.some((each) => each.id !== input.id && each.name === name)) {
      clash("price_list_name_taken");
    }
    const id = input.id ?? fakeId("01FAKELIST");
    const saved = { id, name, active: input.active, created_at: Date.now() };
    priceLists = input.id
      ? priceLists.map((each) =>
          each.id === id ? { ...saved, created_at: each.created_at } : each,
        )
      : [...priceLists, saved];
    recordFakeEvent("pricing.change", currentSession()?.user_id ?? null);
    return id;
  },
  price_list_items: (args?: Args): PriceListItemRow[] => {
    requirePermission("pricing_manage");
    const { products } = fakeCatalog();
    return (listItems[String(args?.id)] ?? []).map((item) => {
      const product = products.find((each) => each.id === item.product_id);
      return {
        product_id: item.product_id,
        name: product?.name ?? "",
        sku: product?.sku ?? "",
        retail_price: product?.sale_price ?? 0,
        price: item.price,
      };
    });
  },
  price_list_set_items: (args?: Args): null => {
    requirePermission("pricing_manage");
    const id = String(args?.id);
    const items = args?.items as PriceListItem[];
    if (items.some((each) => each.price < 0)) {
      refuse("price", "price_negative");
    }
    const kept = (listItems[id] ?? []).filter(
      (old) => !items.some((each) => each.product_id === old.product_id),
    );
    listItems[id] = [...kept, ...items];
    recordFakeEvent("pricing.change", currentSession()?.user_id ?? null);
    return null;
  },
  price_list_remove_items: (args?: Args): null => {
    requirePermission("pricing_manage");
    const id = String(args?.id);
    const gone = args?.productIds as string[];
    listItems[id] = (listItems[id] ?? []).filter((each) => !gone.includes(each.product_id));
    recordFakeEvent("pricing.change", currentSession()?.user_id ?? null);
    return null;
  },
  tier_list: (): Tier[] => {
    requirePermission("pricing_manage");
    return structuredClone(tiers);
  },
  tier_save: (args?: Args): string => {
    requirePermission("pricing_manage");
    const input = args?.input as TierSave;
    if (input.name.trim() === "") {
      refuse("name", "tier_name_missing");
    }
    if (input.discount > 10000) {
      refuse("discount", "discount_over_100");
    }
    const id = input.id ?? fakeId("01FAKETIER");
    const saved: Tier = { ...input, id, name: input.name.trim(), archived_at: null };
    tiers = input.id ? tiers.map((each) => (each.id === id ? saved : each)) : [...tiers, saved];
    recordFakeEvent("pricing.change", currentSession()?.user_id ?? null);
    return id;
  },
  tier_archive: (args?: Args): null => {
    requirePermission("pricing_manage");
    tiers = tiers.map((each) =>
      each.id === args?.id ? { ...each, archived_at: Date.now() } : each,
    );
    recordFakeEvent("pricing.change", currentSession()?.user_id ?? null);
    return null;
  },
  promotion_list: (): Promotion[] => {
    requirePermission("pricing_manage");
    return structuredClone(promotions);
  },
  promotion_save: (args?: Args): string => {
    requirePermission("pricing_manage");
    const input = args?.input as PromotionSave;
    const promotion = input.promotion;
    if (promotion.name.trim() === "") {
      refuse("name", "promotion_name_missing");
    }
    if (promotion.targets.length === 0) {
      refuse("targets", "promotion_targets_missing");
    }
    const id = input.id ?? fakeId("01FAKEPROMO");
    const saved: Promotion = { ...promotion, id, active: true, created_at: Date.now() };
    promotions = input.id
      ? promotions.map((each) => (each.id === id ? { ...saved, active: each.active } : each))
      : [...promotions, saved];
    recordFakeEvent("pricing.change", currentSession()?.user_id ?? null);
    return id;
  },
  promotion_set_active: (args?: Args): null => {
    requirePermission("pricing_manage");
    promotions = promotions.map((each) =>
      each.id === args?.id ? { ...each, active: Boolean(args?.active) } : each,
    );
    recordFakeEvent("pricing.change", currentSession()?.user_id ?? null);
    return null;
  },
  customer_lookup: (args?: Args): CustomerRow[] => {
    requirePermission("customer_view_create");
    return customers
      .filter((each) => each.customer.archived_at === null && matches(each, String(args?.text)))
      .slice(0, 20)
      .map(row);
  },
  customer_list: (args?: Args): CustomerRow[] => {
    requirePermission("customer_view_create");
    return customers
      .filter(
        (each) =>
          (Boolean(args?.includeArchived) || each.customer.archived_at === null) &&
          matches(each, String(args?.text)),
      )
      .sort((a, b) => a.customer.name.localeCompare(b.customer.name))
      .map(row);
  },
  customer_get: (args?: Args): CustomerPage => {
    requirePermission("customer_view_create");
    const each = found(args?.id);
    const sales = each.purchases.filter((one) => !one.voided);
    return {
      customer: structuredClone(each.customer),
      tier_name: tierName(each.customer.tier_id),
      points: pointsOf(each),
      credit: creditOf(each),
      oldest_unpaid_at: creditOf(each) > 0 ? (each.credit[0]?.at ?? null) : null,
      total_spend: sales.reduce(
        (sum, one) => sum + (one.kind === "sale" ? one.total : -one.total),
        0,
      ),
      visits: sales.filter((one) => one.kind === "sale").length,
      purchases: structuredClone(each.purchases),
      points_history: [...each.points].reverse(),
    };
  },
  customer_save: (args?: Args): string => {
    requirePermission("customer_view_create");
    const input = args?.input as CustomerSave;
    if (input.id) {
      requirePermission("customer_manage");
    }
    const details = input.details;
    if (details.name.trim() === "") {
      refuse("name", "customer_name_missing");
    }
    const phone = cleanPhone(details.phone);
    const sameName = customers.find((each) => each.customer.phone === phone);
    if (sameName && sameName.customer.id !== input.id) {
      clash("phone_in_use", { name: sameName.customer.name });
    }
    if (details.credit_limit < 0) {
      refuse("credit_limit", "credit_limit_negative");
    }
    const manager = currentSession()?.permissions.includes("customer_manage") ?? false;
    const now = Date.now();
    if (input.id) {
      const each = found(input.id);
      each.customer = { ...each.customer, ...details, phone, updated_at: now };
      recordFakeEvent("customer.change", currentSession()?.user_id ?? null);
      return input.id;
    }
    const id = fakeId("01FAKECUST");
    addFake(id, phone, details.name.trim(), manager ? details.tier_id : null);
    const added = found(id);
    added.customer = {
      ...added.customer,
      email: details.email,
      address: details.address,
      notes: details.notes,
      loyalty_opt_out: details.loyalty_opt_out,
      credit_limit: manager ? details.credit_limit : 0,
      created_at: now,
      updated_at: now,
    };
    recordFakeEvent("customer.create", currentSession()?.user_id ?? null);
    return id;
  },
  customer_archive: (args?: Args): null => {
    requirePermission("customer_manage");
    const each = found(args?.id);
    each.customer.archived_at = args?.archived ? Date.now() : null;
    recordFakeEvent("customer.change", currentSession()?.user_id ?? null);
    return null;
  },
  customer_payment: (args?: Args): CustomerPaymentDone => {
    requirePermission("customer_receive_payment");
    const input = args?.input as CustomerPaymentInput;
    const each = found(input.customer_id);
    if (input.amount <= 0) {
      refuse("amount", "amount_not_positive");
    }
    const owed = Math.max(creditOf(each), 0);
    if (input.amount > owed) {
      clash("payment_more_than_debt", { owed_minor: String(owed) });
    }
    const isCash = input.method_id === "00000000000000000000000001";
    if (isCash && !tillCash(input.amount)) {
      clash("shift_not_open");
    }
    addCredit(each, -input.amount, "payment", isCash ? "cash" : "other", null);
    recordFakeEvent("customer.payment", currentSession()?.user_id ?? null);
    return { credit: creditOf(each), cash_taken: isCash };
  },
  points_adjust: (args?: Args): number => {
    requirePermission("loyalty_adjust");
    const each = found(args?.customerId);
    const reason = String(args?.reason).trim();
    if (reason === "") {
      refuse("reason", "reason_missing");
    }
    const points = Number(args?.points);
    if (pointsOf(each) + points < 0) {
      clash("points_not_enough", { balance: String(pointsOf(each)) });
    }
    each.points.push({ at: Date.now(), kind: "adjust", points, reason });
    recordFakeEvent("points.adjust", currentSession()?.user_id ?? null);
    return pointsOf(each);
  },
  customer_statement: (args?: Args): Statement => {
    requirePermission("customer_view_create");
    const each = found(args?.id);
    const from = String(args?.from);
    const to = String(args?.to);
    const inRange = each.credit.filter(
      (line) => line.business_date >= from && line.business_date <= to,
    );
    const before = each.credit.filter((line) => line.business_date < from);
    const opening = before.at(-1)?.balance_after ?? 0;
    return {
      customer_id: each.customer.id,
      from,
      to,
      opening,
      lines: structuredClone(inRange),
      closing: inRange.at(-1)?.balance_after ?? opening,
    };
  },
} satisfies Record<string, (args?: Args) => unknown>;
