import type { AdjustInput } from "../../generated/AdjustInput";
import type { Category } from "../../generated/Category";
import type { CountInput } from "../../generated/CountInput";
import type { CountLineInput } from "../../generated/CountLineInput";
import type { CountPosted } from "../../generated/CountPosted";
import type { CountSummary } from "../../generated/CountSummary";
import type { CountView } from "../../generated/CountView";
import type { HistoryQuery } from "../../generated/HistoryQuery";
import type { HistoryRow } from "../../generated/HistoryRow";
import type { MismatchView } from "../../generated/MismatchView";
import type { ProductPage } from "../../generated/ProductPage";
import type { Reason } from "../../generated/Reason";
import type { StockAlerts } from "../../generated/StockAlerts";
import type { StockFilter } from "../../generated/StockFilter";
import type { StockList } from "../../generated/StockList";
import type { StockProduct } from "../../generated/StockProduct";
import type { VarianceReport } from "../../generated/VarianceReport";
import { call } from "../../shared/bridge";

// One page of stock levels with value, by name (INV-01, INV-03, INV-04, INV-05, INV-08).
export const listStock = (filter: StockFilter, page: ProductPage) =>
  call<StockList>("stock_levels", { filter, page });
// One product's stock, reorder rule and batches.
export const getStock = (id: string) => call<StockProduct>("stock_product", { id });
// Finds a product by barcode or SKU, with its stock (scanning on the count screen).
export const lookupStock = (code: string) => call<StockProduct>("stock_lookup", { code });
// A product's stock history, newest first, with the balance after each row (INV-07).
export const stockHistory = (query: HistoryQuery) => call<HistoryRow[]>("stock_history", { query });
// The active adjustment reasons (INV-02).
export const stockReasons = () => call<Reason[]>("stock_reasons");
// Adjusts stock with a reason. Returns the stock afterwards.
export const adjustStock = (input: AdjustInput) => call<number>("stock_adjust", { input });
// Low stock, near expiry and negative stock for the dashboard.
export const stockAlerts = () => call<StockAlerts>("stock_alerts");
// "Verify stock data" (INV-10): levels that don't match their ledger. Nothing is fixed.
export const verifyStock = () => call<MismatchView[]>("stock_verify");
// The categories for the filter (read through the catalog command).
export const listCategories = () => call<Category[]>("category_list", { includeArchived: false });

export const listCounts = (page: ProductPage) => call<CountSummary[]>("count_list", { page });
export const getCount = (id: string) => call<CountView>("count_get", { id });
export const createCount = (input: CountInput) => call<string>("count_create", { input });
// Adds one scan or typed quantity; a negative quantity takes back a wrong scan.
export const countLine = (countId: string, line: CountLineInput) =>
  call<CountView>("count_line", { countId, line });
export const countVariances = (id: string) => call<VarianceReport>("count_variances", { id });
// Makes a backup, then posts the differences. Uncounted products in a full count need confirming.
export const postCount = (id: string, confirmUncounted: boolean) =>
  call<CountPosted>("count_post", { id, confirmUncounted });
export const cancelCount = (id: string) => call<null>("count_cancel", { id });
