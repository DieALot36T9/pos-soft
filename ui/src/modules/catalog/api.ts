import type { Brand } from "../../generated/Brand";
import type { BrandInput } from "../../generated/BrandInput";
import type { Category } from "../../generated/Category";
import type { CategoryInput } from "../../generated/CategoryInput";
import type { KitPartInput } from "../../generated/KitPartInput";
import type { ProductChoices } from "../../generated/ProductChoices";
import type { ProductDetails } from "../../generated/ProductDetails";
import type { ProductFilter } from "../../generated/ProductFilter";
import type { ProductInput } from "../../generated/ProductInput";
import type { ProductList } from "../../generated/ProductList";
import type { ProductPage } from "../../generated/ProductPage";
import type { SearchHit } from "../../generated/SearchHit";
import type { Unit } from "../../generated/Unit";
import type { UnitInput } from "../../generated/UnitInput";
import type { VariantAttribute } from "../../generated/VariantAttribute";
import { call } from "../../shared/bridge";

// One page of products, sorted by name (archived ones only when the filter asks).
export const listProducts = (filter: ProductFilter, page: ProductPage) =>
  call<ProductList>("product_list", { filter, page });
// Search by name, SKU, barcode, category or brand; exact barcode or SKU first (CAT-05).
export const searchProducts = (text: string, limit: number) =>
  call<SearchHit[]>("product_search", { text, limit });
// Everything about one product: fields, reorder rule, stock, kit parts, variants, price history, picture.
export const getProduct = (id: string) => call<ProductDetails>("product_get", { id });
// The units, categories, brands, tax rates, suppliers and money format the form picks from.
export const productChoices = () => call<ProductChoices>("product_choices");
// Adds (no id) or changes a product with its barcodes and reorder rule. Returns its id.
export const saveProduct = (input: ProductInput) => call<string>("product_save", { input });
export const archiveProduct = (id: string) => call<null>("product_archive", { id });
export const unarchiveProduct = (id: string) => call<null>("product_unarchive", { id });
// Refused once the product was used anywhere (CAT-08).
export const deleteProduct = (id: string) => call<null>("product_delete", { id });
// Adds an internal EAN-13 from the shop's prefix (CAT-11). Returns the new code.
export const generateBarcode = (id: string) => call<string>("barcode_generate", { id });
export const generateVariants = (parent: string, attributes: VariantAttribute[]) =>
  call<string[]>("variants_generate", { parent, attributes });
export const saveKit = (kit: string, parts: KitPartInput[]) =>
  call<null>("kit_save", { kit, parts });
// The file is a path from the picture picker; Rust reads and resizes it (CAT-10).
export const setProductImage = (id: string, file: string) =>
  call<null>("product_image_set", { id, file });

export const listUnits = (includeArchived: boolean) =>
  call<Unit[]>("unit_list", { includeArchived });
export const saveUnit = (input: UnitInput) => call<string>("unit_save", { input });
export const archiveUnit = (id: string) => call<null>("unit_archive", { id });
export const listCategories = (includeArchived: boolean) =>
  call<Category[]>("category_list", { includeArchived });
export const saveCategory = (input: CategoryInput) => call<string>("category_save", { input });
export const archiveCategory = (id: string) => call<null>("category_archive", { id });
export const listBrands = (includeArchived: boolean) =>
  call<Brand[]>("brand_list", { includeArchived });
export const saveBrand = (input: BrandInput) => call<string>("brand_save", { input });
export const archiveBrand = (id: string) => call<null>("brand_archive", { id });
