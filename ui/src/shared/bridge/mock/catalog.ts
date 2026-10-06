import type { AppError } from "../../../generated/AppError";
import type { Brand } from "../../../generated/Brand";
import type { BrandInput } from "../../../generated/BrandInput";
import type { Category } from "../../../generated/Category";
import type { CategoryInput } from "../../../generated/CategoryInput";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { KitPartInput } from "../../../generated/KitPartInput";
import type { PriceHistoryView } from "../../../generated/PriceHistoryView";
import type { Product } from "../../../generated/Product";
import type { ProductChoices } from "../../../generated/ProductChoices";
import type { ProductDetails } from "../../../generated/ProductDetails";
import type { ProductFilter } from "../../../generated/ProductFilter";
import type { ProductInput } from "../../../generated/ProductInput";
import type { ProductList } from "../../../generated/ProductList";
import type { ProductPage } from "../../../generated/ProductPage";
import type { ProductRow } from "../../../generated/ProductRow";
import type { ReorderInput } from "../../../generated/ReorderInput";
import type { SearchHit } from "../../../generated/SearchHit";
import type { Unit } from "../../../generated/Unit";
import type { UnitInput } from "../../../generated/UnitInput";
import type { VariantAttribute } from "../../../generated/VariantAttribute";
import { currentSession, recordFakeEvent, requirePermission } from "./people";

// A pretend catalog for the fake bridge. Rust does the real checks (Module - catalog);
// this stand-in checks only what the screens and their tests need, with the same error codes.

type Args = Record<string, unknown>;

// What the catalog needs from the pretend shop: its tax rates and money format.
type ShopFacts = {
  rates: () => { id: string; name: string; active: boolean }[];
  defaultRate: () => string | null;
  currency: () => CurrencyFormat;
};

let facts: ShopFacts = {
  rates: () => [],
  defaultRate: () => null,
  currency: () => {
    throw new Error("The fake catalog isn't connected to the fake shop.");
  },
};

// shop.ts calls this once, so the catalog can read the shop's rates and currency.
export function connectCatalog(shop: ShopFacts) {
  facts = shop;
}

const UNCATEGORISED = "00000000000000000000000010";
const PIECE = "00000000000000000000000001";

let units: Unit[] = [];
let categories: Category[] = [];
let brands: Brand[] = [];
let products: Product[] = [];
let reorder = new Map<string, ReorderInput>();
let kits = new Map<string, KitPartInput[]>();
let history = new Map<string, PriceHistoryView[]>();
let images = new Map<string, string>();
let nextSku = 1;
let nextInternal = 1;
let nextId = 1;

function fakeId(prefix: string) {
  const number = String(nextId++);
  return prefix + number.padStart(26 - prefix.length, "0");
}

function unit(id: string, name: string, symbol: string, decimals: number): Unit {
  return { id, name, symbol, unit_code: null, decimals, is_system: true, archived_at: null };
}

// Starts the pretend catalog again. A set-up shop has a few demo products.
export function resetCatalog(withDemoProducts: boolean) {
  units = [
    unit(PIECE, "Piece", "pc", 0),
    unit("00000000000000000000000002", "Kilogram", "kg", 3),
    unit("00000000000000000000000003", "Gram", "g", 0),
    unit("00000000000000000000000004", "Litre", "L", 3),
    unit("00000000000000000000000005", "Millilitre", "ml", 0),
    unit("00000000000000000000000006", "Pack", "pack", 0),
  ];
  categories = [
    { id: UNCATEGORISED, parent_id: null, name: "Uncategorised", sort: 0, archived_at: null },
  ];
  brands = [];
  products = [];
  reorder = new Map();
  kits = new Map();
  history = new Map();
  images = new Map();
  nextSku = 1;
  nextInternal = 1;
  nextId = 1;
  if (withDemoProducts) {
    const dairy = addCategory("Dairy", null);
    const brand = addBrand("Farm Fresh");
    addDemo("Milk 1L", 9000, 7500, dairy, brand, ["8901234567890"]);
    addDemo("Bread", 5000, 3800, UNCATEGORISED, null, []);
    addDemo("Soap", 3000, 2200, UNCATEGORISED, null, ["8902222222227"]);
  }
}

function addCategory(name: string, parent: string | null) {
  const id = fakeId("01FAKECAT");
  categories.push({ id, parent_id: parent, name, sort: 0, archived_at: null });
  return id;
}

function addBrand(name: string) {
  const id = fakeId("01FAKEBRAND");
  brands.push({ id, name, archived_at: null });
  return id;
}

function addDemo(
  name: string,
  sale: number,
  cost: number,
  category: string,
  brand: string | null,
  barcodes: string[],
) {
  create({
    ...blankInput(name),
    sale_price: sale,
    cost_price: cost,
    category_id: category,
    brand_id: brand,
    barcodes,
  });
}

function blankInput(name: string): ProductInput {
  return {
    id: null,
    kind: "standard",
    name,
    sku: "",
    category_id: null,
    brand_id: null,
    unit_id: PIECE,
    sale_price: 0,
    cost_price: 0,
    tax_rate_id: null,
    is_weighed: false,
    scale_item_code: null,
    track_stock: true,
    track_expiry: false,
    preferred_supplier_id: null,
    description: null,
    barcodes: [],
    reorder: null,
  };
}

function fail(error: AppError): never {
  // Rust's invoke fails with a plain AppError object, so the fake does too.
  // eslint-disable-next-line @typescript-eslint/only-throw-error
  throw error;
}

function refuse(field: string | null, code: string, params: Record<string, string> = {}): never {
  return fail({ kind: "validation", field, code, params });
}

function clash(code: string, params: Record<string, string>): never {
  return fail({ kind: "conflict", code, params });
}

function notFound(entity: string, key: string): never {
  return fail({ kind: "not_found", entity, key });
}

function product(id: string): Product {
  return products.find((each) => each.id === id) ?? notFound("product", id);
}

// The GS1 check digit, as barcode::is_valid_gtin works it out.
function validGtin(code: string) {
  const body = code.slice(0, -1);
  let sum = 0;
  [...body].reverse().forEach((digit, index) => {
    sum += Number(digit) * (index % 2 === 0 ? 3 : 1);
  });
  return (10 - (sum % 10)) % 10 === Number(code.slice(-1));
}

function cleanBarcodes(codes: string[]) {
  const clean: string[] = [];
  for (const raw of codes) {
    const code = raw.trim();
    if (code === "" || clean.includes(code)) {
      continue;
    }
    if (/^\d+$/.test(code) && [8, 12, 13].includes(code.length) && !validGtin(code)) {
      refuse("barcodes", "barcode_check_digit_wrong", { code });
    }
    clean.push(code);
  }
  return clean;
}

function checkBarcodesFree(codes: string[], owner: string | null) {
  for (const code of codes) {
    const other = products.find((each) => each.id !== owner && each.barcodes.includes(code));
    if (other) {
      clash("barcode_in_use", { code, product_name: other.name, sku: other.sku });
    }
  }
}

function checkFields(input: ProductInput, owner: string | null) {
  const name = input.name.trim();
  if (name === "") {
    refuse("name", "name_missing");
  }
  if (name.length > 120) {
    refuse("name", "name_too_long");
  }
  if (input.sale_price < 0 || input.cost_price < 0) {
    refuse("sale_price", "price_negative");
  }
  const sku = input.sku.trim();
  if (sku !== "" && products.some((each) => each.id !== owner && each.sku === sku)) {
    clash("sku_in_use", { value: sku });
  }
  const chosenUnit = units.find((each) => each.id === input.unit_id);
  if (input.is_weighed && chosenUnit?.decimals === 0) {
    refuse("is_weighed", "weighed_unit_needs_decimals");
  }
  if (input.reorder && (input.reorder.level < 0 || input.reorder.qty < 0)) {
    refuse("reorder", "reorder_below_zero");
  }
}

function needsPricePermission(input: ProductInput) {
  if (!input.id) {
    return input.sale_price !== 0 || input.cost_price !== 0;
  }
  const saved = product(input.id);
  return saved.sale_price !== input.sale_price || saved.cost_price !== input.cost_price;
}

function noStock(kind: Product["kind"]) {
  return kind === "kit" || kind === "variant_parent";
}

function create(input: ProductInput): string {
  checkFields(input, null);
  const barcodes = cleanBarcodes(input.barcodes);
  checkBarcodesFree(barcodes, null);
  let sku = input.sku.trim();
  if (sku === "") {
    do {
      sku = `P${String(nextSku++).padStart(6, "0")}`;
    } while (products.some((each) => each.sku === sku));
  }
  const id = fakeId("01FAKEPRODUCT");
  products.push({
    id,
    kind: input.kind,
    parent_id: null,
    variant_label: null,
    name: input.name.trim(),
    sku,
    category_id: input.category_id ?? UNCATEGORISED,
    brand_id: input.brand_id,
    unit_id: input.unit_id,
    sale_price: input.sale_price,
    cost_price: input.cost_price,
    tax_rate_id: input.tax_rate_id ?? facts.defaultRate() ?? "",
    is_weighed: input.is_weighed,
    scale_item_code: input.scale_item_code?.trim() || null,
    track_stock: noStock(input.kind) ? false : input.track_stock,
    track_expiry: input.track_expiry,
    preferred_supplier_id: input.preferred_supplier_id,
    image_path: null,
    description: input.description?.trim() || null,
    archived_at: null,
    created_at: Date.now(),
    updated_at: Date.now(),
    barcodes,
    attributes: [],
  });
  saveReorder(id, input.reorder, !noStock(input.kind) && input.track_stock);
  return id;
}

function saveReorder(id: string, rule: ReorderInput | null, tracked: boolean) {
  if (rule && tracked) {
    reorder.set(id, rule);
  } else {
    reorder.delete(id);
  }
}

function update(input: ProductInput & { id: string }) {
  const saved = product(input.id);
  checkFields(input, saved.id);
  if (input.sku.trim() === "") {
    refuse("sku", "sku_missing");
  }
  const barcodes = cleanBarcodes(input.barcodes);
  checkBarcodesFree(barcodes, saved.id);
  const changes: PriceHistoryView[] = [];
  const who = currentSession()?.display_name ?? null;
  if (saved.sale_price !== input.sale_price) {
    changes.push(priceRow("sale_price", saved.sale_price, input.sale_price, who));
  }
  if (saved.cost_price !== input.cost_price) {
    changes.push(priceRow("cost_price", saved.cost_price, input.cost_price, who));
  }
  history.set(saved.id, [...(history.get(saved.id) ?? []), ...changes]);
  Object.assign(saved, {
    name: input.name.trim(),
    sku: input.sku.trim(),
    category_id: input.category_id ?? UNCATEGORISED,
    brand_id: input.brand_id,
    unit_id: input.unit_id,
    sale_price: input.sale_price,
    cost_price: input.cost_price,
    tax_rate_id: input.tax_rate_id ?? saved.tax_rate_id,
    is_weighed: input.is_weighed,
    scale_item_code: input.scale_item_code?.trim() || null,
    track_stock: noStock(saved.kind) ? false : input.track_stock,
    track_expiry: input.track_expiry,
    preferred_supplier_id: input.preferred_supplier_id,
    description: input.description?.trim() || null,
    barcodes,
    updated_at: Date.now(),
  });
  saveReorder(saved.id, input.reorder, saved.track_stock);
}

function priceRow(
  field: PriceHistoryView["field"],
  old: number,
  next: number,
  who: string | null,
): PriceHistoryView {
  return { field, old, new: next, changed_at: Date.now(), changed_by_name: who, reason: "manual" };
}

function row(each: Product): ProductRow {
  return {
    id: each.id,
    kind: each.kind,
    name: each.name,
    sku: each.sku,
    category_id: each.category_id,
    category_name: categories.find((category) => category.id === each.category_id)?.name ?? "",
    brand_name: brands.find((brand) => brand.id === each.brand_id)?.name ?? null,
    unit_symbol: units.find((one) => one.id === each.unit_id)?.symbol ?? "",
    sale_price: each.sale_price,
    cost_price: each.cost_price,
    archived_at: each.archived_at,
  };
}

// The category and every category below it.
function categoryAndBelow(id: string): string[] {
  const below = categories.filter((each) => each.parent_id === id).map((each) => each.id);
  return [id, ...below.flatMap(categoryAndBelow)];
}

function listRows(filter: ProductFilter, page: ProductPage): ProductList {
  const text = filter.text?.trim().toLowerCase() ?? "";
  const inCategory = filter.category_id ? categoryAndBelow(filter.category_id) : null;
  const matching = products
    .filter((each) => filter.include_archived || each.archived_at === null)
    .filter((each) => !filter.kind || each.kind === filter.kind)
    .filter((each) => !filter.brand_id || each.brand_id === filter.brand_id)
    .filter((each) => !filter.parent_id || each.parent_id === filter.parent_id)
    .filter((each) => !inCategory || inCategory.includes(each.category_id))
    .filter(
      (each) =>
        text === "" ||
        each.name.toLowerCase().includes(text) ||
        each.sku.toLowerCase().includes(text),
    )
    .sort((a, b) => a.name.localeCompare(b.name));
  return {
    rows: matching.slice(page.offset, page.offset + page.limit).map(row),
    total: matching.length,
  };
}

function search(text: string, limit: number): SearchHit[] {
  const words = text.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) {
    return [];
  }
  const hits = products
    .filter((each) => each.archived_at === null && each.kind !== "variant_parent")
    .map((each) => {
      const exact = each.barcodes.includes(text.trim()) || each.sku === text.trim();
      const haystack = `${each.name} ${each.sku} ${each.barcodes.join(" ")}`.toLowerCase();
      const found = exact || words.every((word) => haystack.includes(word));
      return { each, exact, found };
    })
    .filter((hit) => hit.found)
    .sort((a, b) => Number(b.exact) - Number(a.exact) || a.each.name.localeCompare(b.each.name));
  return hits.slice(0, limit).map(({ each, exact }) => ({
    product_id: each.id,
    name: each.name,
    sku: each.sku,
    sale_price: each.sale_price,
    unit_symbol: units.find((one) => one.id === each.unit_id)?.symbol ?? "",
    thumbnail: null,
    exact_match: exact,
  }));
}

function usedInKit(id: string) {
  return [...kits.values()].some((parts) => parts.some((part) => part.product_id === id));
}

function internalCode(number: number) {
  const first12 = `200${String(number).padStart(9, "0")}`;
  for (let check = 0; check < 10; check += 1) {
    if (validGtin(first12 + String(check))) {
      return first12 + String(check);
    }
  }
  return first12;
}

function cartesian(attributes: VariantAttribute[]): [string, string][][] {
  return attributes.reduce<[string, string][][]>(
    (combos, attribute) =>
      combos.flatMap((combo) =>
        attribute.values.map((value) => [...combo, [attribute.name, value] as [string, string]]),
      ),
    [[]],
  );
}

function audit(action: string) {
  recordFakeEvent(action, currentSession()?.user_id ?? null);
}

// A tiny picture for the fake bridge: an orange square.
const FAKE_PICTURE =
  "data:image/svg+xml;utf8," +
  encodeURIComponent(
    '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" fill="#F2B544"/></svg>',
  );

export const catalogHandlers = {
  product_list: (args?: Args): ProductList => {
    requirePermission("stock_view");
    return listRows(args?.filter as ProductFilter, args?.page as ProductPage);
  },
  product_search: (args?: Args): SearchHit[] => {
    requirePermission("stock_view");
    return search(typeof args?.text === "string" ? args.text : "", Number(args?.limit ?? 20));
  },
  product_get: (args?: Args): ProductDetails => {
    requirePermission("stock_view");
    const found = product(String(args?.id));
    const parts = kits.get(found.id) ?? [];
    return {
      product: structuredClone(found),
      reorder: reorder.get(found.id) ?? null,
      on_hand: 0,
      kit_parts: parts.map((part) => {
        const each = product(part.product_id);
        return {
          product_id: each.id,
          name: each.name,
          sku: each.sku,
          unit_symbol: units.find((one) => one.id === each.unit_id)?.symbol ?? "",
          qty: part.qty,
        };
      }),
      variants: products.filter((each) => each.parent_id === found.id).map(row),
      price_history: history.get(found.id) ?? [],
      image: images.get(found.id) ?? null,
    };
  },
  product_choices: (): ProductChoices => {
    requirePermission("stock_view");
    return {
      units: units.filter((each) => each.archived_at === null),
      categories: categories.filter((each) => each.archived_at === null),
      brands: brands.filter((each) => each.archived_at === null),
      tax_rates: facts
        .rates()
        .filter((each) => each.active)
        .map((each) => ({ id: each.id, name: each.name })),
      suppliers: [],
      default_tax_rate_id: facts.defaultRate(),
      currency: facts.currency(),
    };
  },
  product_save: (args?: Args): string => {
    requirePermission("catalog_manage");
    const input = args?.input as ProductInput;
    if (needsPricePermission(input)) {
      requirePermission("catalog_price_edit");
    }
    if (input.id) {
      update({ ...input, id: input.id });
      audit("product.change");
      return input.id;
    }
    const id = create(input);
    audit("product.create");
    return id;
  },
  product_archive: (args?: Args): null => {
    requirePermission("catalog_manage");
    product(String(args?.id)).archived_at = Date.now();
    audit("product.archive");
    return null;
  },
  product_unarchive: (args?: Args): null => {
    requirePermission("catalog_manage");
    product(String(args?.id)).archived_at = null;
    audit("product.unarchive");
    return null;
  },
  product_delete: (args?: Args): null => {
    requirePermission("catalog_manage");
    const found = product(String(args?.id));
    // The demo products count as sold, so they can only be archived.
    const demo = ["Milk 1L", "Bread", "Soap"].includes(found.name);
    if (demo || usedInKit(found.id)) {
      clash("product_in_use", { value: found.name });
    }
    if (products.some((each) => each.parent_id === found.id)) {
      clash("product_has_variants", { value: found.name });
    }
    products = products.filter((each) => each.id !== found.id);
    reorder.delete(found.id);
    audit("product.delete");
    return null;
  },
  barcode_generate: (args?: Args): string => {
    requirePermission("catalog_manage");
    const found = product(String(args?.id));
    if (found.kind === "variant_parent") {
      refuse("barcode", "parent_not_sellable");
    }
    let code = internalCode(nextInternal++);
    while (products.some((each) => each.barcodes.includes(code))) {
      code = internalCode(nextInternal++);
    }
    found.barcodes.push(code);
    audit("product.change");
    return code;
  },
  variants_generate: (args?: Args): string[] => {
    requirePermission("catalog_manage");
    const parent = product(String(args?.parent));
    if (parent.kind !== "variant_parent") {
      refuse("parent_id", "not_a_variant_parent");
    }
    const attributes = (args?.attributes as VariantAttribute[]).map((each) => ({
      name: each.name.trim(),
      values: each.values.map((value) => value.trim()).filter(Boolean),
    }));
    if (attributes.length === 0 || attributes.some((each) => !each.name || !each.values.length)) {
      refuse("attributes", "variant_attribute_missing");
    }
    const made: string[] = [];
    for (const combo of cartesian(attributes)) {
      const label = combo.map(([, value]) => value).join(" / ");
      if (products.some((each) => each.parent_id === parent.id && each.variant_label === label)) {
        continue;
      }
      const id = fakeId("01FAKEVARIANT");
      products.push({
        ...structuredClone(parent),
        id,
        kind: "variant",
        parent_id: parent.id,
        variant_label: label,
        name: `${parent.name} ${combo.map(([, value]) => value).join(" ")}`,
        sku: `P${String(nextSku++).padStart(6, "0")}`,
        track_stock: true,
        barcodes: [],
        attributes: combo,
      });
      made.push(id);
    }
    audit("product.change");
    return made;
  },
  kit_save: (args?: Args): null => {
    requirePermission("catalog_manage");
    const kit = product(String(args?.kit));
    if (kit.kind !== "kit") {
      refuse("kit_id", "not_a_kit");
    }
    const parts = args?.parts as KitPartInput[];
    for (const part of parts) {
      if (part.product_id === kit.id) {
        refuse("components", "kit_contains_itself");
      }
      if (part.qty <= 0) {
        refuse("components", "kit_qty_not_positive");
      }
    }
    kits.set(kit.id, structuredClone(parts));
    audit("product.change");
    return null;
  },
  product_image_set: (args?: Args): null => {
    requirePermission("catalog_manage");
    const found = product(String(args?.id));
    found.image_path = `${found.id}.jpg`;
    images.set(found.id, FAKE_PICTURE);
    audit("product.change");
    return null;
  },
  unit_list: (args?: Args): Unit[] => {
    requirePermission("stock_view");
    return units.filter((each) => args?.includeArchived === true || each.archived_at === null);
  },
  unit_save: (args?: Args): string => {
    requirePermission("catalog_manage");
    const input = args?.input as UnitInput;
    if (input.fields.name.trim() === "") {
      refuse("name", "name_missing");
    }
    if (input.fields.symbol.trim() === "") {
      refuse("symbol", "symbol_missing");
    }
    const sameName = units.find(
      (each) =>
        each.id !== input.id && each.name.toLowerCase() === input.fields.name.trim().toLowerCase(),
    );
    if (sameName) {
      clash("unit_name_in_use", { value: sameName.name });
    }
    audit("unit.change");
    if (input.id) {
      const found = units.find((each) => each.id === input.id) ?? notFound("unit", input.id);
      Object.assign(found, input.fields);
      return found.id;
    }
    const id = fakeId("01FAKEUNIT");
    units.push({ id, ...input.fields, is_system: false, archived_at: null });
    return id;
  },
  unit_archive: (args?: Args): null => {
    requirePermission("catalog_manage");
    const found = units.find((each) => each.id === args?.id) ?? notFound("unit", String(args?.id));
    if (found.is_system) {
      clash("unit_is_built_in", { value: found.name });
    }
    if (products.some((each) => each.unit_id === found.id && each.archived_at === null)) {
      clash("unit_in_use", { value: found.name });
    }
    found.archived_at = Date.now();
    audit("unit.change");
    return null;
  },
  category_list: (args?: Args): Category[] => {
    requirePermission("stock_view");
    return categories.filter((each) => args?.includeArchived === true || each.archived_at === null);
  },
  category_save: (args?: Args): string => {
    requirePermission("catalog_manage");
    const input = args?.input as CategoryInput;
    if (input.fields.name.trim() === "") {
      refuse("name", "name_missing");
    }
    if (
      input.id &&
      input.fields.parent_id &&
      categoryAndBelow(input.id).includes(input.fields.parent_id)
    ) {
      refuse("parent_id", "category_parent_loop");
    }
    audit("category.change");
    if (input.id) {
      const found =
        categories.find((each) => each.id === input.id) ?? notFound("category", input.id);
      Object.assign(found, { ...input.fields, name: input.fields.name.trim() });
      return found.id;
    }
    const id = addCategory(input.fields.name.trim(), input.fields.parent_id);
    const added = categories.find((each) => each.id === id);
    if (added) {
      added.sort = input.fields.sort;
    }
    return id;
  },
  category_archive: (args?: Args): null => {
    requirePermission("catalog_manage");
    const found =
      categories.find((each) => each.id === args?.id) ?? notFound("category", String(args?.id));
    if (found.id === UNCATEGORISED) {
      clash("category_is_built_in", { value: found.name });
    }
    if (categories.some((each) => each.parent_id === found.id && each.archived_at === null)) {
      clash("category_has_children", { value: found.name });
    }
    if (products.some((each) => each.category_id === found.id && each.archived_at === null)) {
      clash("category_has_products", { value: found.name });
    }
    found.archived_at = Date.now();
    audit("category.change");
    return null;
  },
  brand_list: (args?: Args): Brand[] => {
    requirePermission("stock_view");
    return brands.filter((each) => args?.includeArchived === true || each.archived_at === null);
  },
  brand_save: (args?: Args): string => {
    requirePermission("catalog_manage");
    const input = args?.input as BrandInput;
    const name = input.name.trim();
    if (name === "") {
      refuse("name", "name_missing");
    }
    const sameName = brands.find(
      (each) => each.id !== input.id && each.name.toLowerCase() === name.toLowerCase(),
    );
    if (sameName) {
      clash("brand_name_in_use", { value: sameName.name });
    }
    audit("brand.change");
    if (input.id) {
      const found = brands.find((each) => each.id === input.id) ?? notFound("brand", input.id);
      found.name = name;
      return found.id;
    }
    return addBrand(name);
  },
  brand_archive: (args?: Args): null => {
    requirePermission("catalog_manage");
    const found =
      brands.find((each) => each.id === args?.id) ?? notFound("brand", String(args?.id));
    found.archived_at = Date.now();
    audit("brand.change");
    return null;
  },
} satisfies Record<string, (args?: Args) => unknown>;
