import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { ProductDetails } from "../../../generated/ProductDetails";
import type { ProductInput } from "../../../generated/ProductInput";
import type { ProductKind } from "../../../generated/ProductKind";
import type { Unit } from "../../../generated/Unit";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { formatQty, parseQty } from "../../../shared/format/qty";

// The product form as typed: every box is text, "" means "none" for the pickers.
// Rust checks the rules; this only turns typed text into numbers.
export type Draft = {
  kind: ProductKind;
  name: string;
  sku: string;
  categoryId: string;
  brandId: string;
  unitId: string;
  salePrice: string;
  costPrice: string;
  // "" means the shop's default rate.
  taxRateId: string;
  isWeighed: boolean;
  scaleCode: string;
  trackStock: boolean;
  trackExpiry: boolean;
  supplierId: string;
  description: string;
  barcodes: string[];
  // Both empty means no reorder rule.
  reorderLevel: string;
  reorderQty: string;
};

// What the editor passes to every tab: the draft and a way to change part of it.
export type DraftProps = {
  draft: Draft;
  change: (part: Partial<Draft>) => void;
  // The message for a field Rust (or the number reader) refused.
  errorFor: (field: string) => string | null;
};

const PIECE = "00000000000000000000000001";
const UNCATEGORISED = "00000000000000000000000010";

// A new product's form: a piece, in "Uncategorised", stock tracked.
export function emptyDraft(): Draft {
  return {
    kind: "standard",
    name: "",
    sku: "",
    categoryId: UNCATEGORISED,
    brandId: "",
    unitId: PIECE,
    salePrice: "",
    costPrice: "",
    taxRateId: "",
    isWeighed: false,
    scaleCode: "",
    trackStock: true,
    trackExpiry: false,
    supplierId: "",
    description: "",
    barcodes: [],
    reorderLevel: "",
    reorderQty: "",
  };
}

// Amounts typed in the boxes have no symbol or thousands sign: "1234.50".
function plainMoney(minor: number, currency: CurrencyFormat) {
  return formatMoney(minor, { ...currency, symbol: "", group_sep: "" });
}

// The form for a saved product.
export function draftOf(details: ProductDetails, currency: CurrencyFormat, units: Unit[]): Draft {
  const product = details.product;
  const decimals = units.find((unit) => unit.id === product.unit_id)?.decimals ?? 0;
  return {
    kind: product.kind,
    name: product.name,
    sku: product.sku,
    categoryId: product.category_id,
    brandId: product.brand_id ?? "",
    unitId: product.unit_id,
    salePrice: plainMoney(product.sale_price, currency),
    costPrice: plainMoney(product.cost_price, currency),
    taxRateId: product.tax_rate_id,
    isWeighed: product.is_weighed,
    scaleCode: product.scale_item_code ?? "",
    trackStock: product.track_stock,
    trackExpiry: product.track_expiry,
    supplierId: product.preferred_supplier_id ?? "",
    description: product.description ?? "",
    barcodes: [...product.barcodes],
    reorderLevel: details.reorder ? formatQty(details.reorder.level, decimals) : "",
    reorderQty: details.reorder ? formatQty(details.reorder.qty, decimals) : "",
  };
}

// The result of reading the form: the input for Rust, or the field and error code to show.
export type ReadDraft =
  | { ok: true; input: ProductInput }
  | { ok: false; field: string; code: string; params?: Record<string, string> };

// Turns the typed form into a ProductInput. Empty prices count as 0.
export function inputOf(
  draft: Draft,
  id: string | null,
  currency: CurrencyFormat,
  units: Unit[],
): ReadDraft {
  const digits = { max_decimals: String(currency.minor_digits) };
  const sale = parseMoney(draft.salePrice === "" ? "0" : draft.salePrice, currency.minor_digits);
  if (!sale.ok) {
    return { ok: false, field: "sale_price", code: sale.code, params: digits };
  }
  const cost = parseMoney(draft.costPrice === "" ? "0" : draft.costPrice, currency.minor_digits);
  if (!cost.ok) {
    return { ok: false, field: "cost_price", code: cost.code, params: digits };
  }
  const decimals = units.find((unit) => unit.id === draft.unitId)?.decimals ?? 0;
  let reorder: ProductInput["reorder"] = null;
  if (draft.reorderLevel.trim() !== "" || draft.reorderQty.trim() !== "") {
    const level = parseQty(draft.reorderLevel, decimals);
    if (!level.ok) {
      return { ok: false, field: "reorder_level", code: level.code };
    }
    const qty = parseQty(draft.reorderQty, decimals);
    if (!qty.ok) {
      return { ok: false, field: "reorder_qty", code: qty.code };
    }
    reorder = { level: level.milli, qty: qty.milli };
  }
  const orNull = (text: string) => (text.trim() === "" ? null : text);
  return {
    ok: true,
    input: {
      id,
      kind: draft.kind,
      name: draft.name,
      sku: draft.sku,
      category_id: orNull(draft.categoryId),
      brand_id: orNull(draft.brandId),
      unit_id: draft.unitId,
      sale_price: sale.minor,
      cost_price: cost.minor,
      tax_rate_id: orNull(draft.taxRateId),
      is_weighed: draft.isWeighed,
      scale_item_code: orNull(draft.scaleCode),
      track_stock: draft.trackStock,
      track_expiry: draft.trackExpiry,
      preferred_supplier_id: orNull(draft.supplierId),
      description: orNull(draft.description),
      barcodes: draft.barcodes,
      reorder,
    },
  };
}
