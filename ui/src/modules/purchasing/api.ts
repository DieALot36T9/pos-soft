import type { AgingView } from "../../generated/AgingView";
import type { Grn } from "../../generated/Grn";
import type { GrnDone } from "../../generated/GrnDone";
import type { GrnInput } from "../../generated/GrnInput";
import type { GrnPreview } from "../../generated/GrnPreview";
import type { GrnRow } from "../../generated/GrnRow";
import type { GrnStart } from "../../generated/GrnStart";
import type { NewOrderLine } from "../../generated/NewOrderLine";
import type { OrderList } from "../../generated/OrderList";
import type { OrderSave } from "../../generated/OrderSave";
import type { OrderStatus } from "../../generated/OrderStatus";
import type { OrderView } from "../../generated/OrderView";
import type { ReturnRow } from "../../generated/ReturnRow";
import type { SearchHit } from "../../generated/SearchHit";
import type { SupplierList } from "../../generated/SupplierList";
import type { SupplierPage } from "../../generated/SupplierPage";
import type { SupplierPaymentDone } from "../../generated/SupplierPaymentDone";
import type { SupplierPaymentInput } from "../../generated/SupplierPaymentInput";
import type { SupplierReturnInput } from "../../generated/SupplierReturnInput";
import type { SupplierReturnRef } from "../../generated/SupplierReturnRef";
import type { SupplierSave } from "../../generated/SupplierSave";
import type { SupplierStatement } from "../../generated/SupplierStatement";
import { call } from "../../shared/bridge";

// Suppliers by name with what the shop owes them; archived ones when asked (PUR-01).
export const listSuppliers = (includeArchived: boolean) =>
  call<SupplierList>("supplier_list", { includeArchived });
// One supplier: details, balance, deliveries and the ways to pay.
export const getSupplier = (id: string) => call<SupplierPage>("supplier_get", { id });
// Adds a supplier (no id) or changes one.
export const saveSupplier = (input: SupplierSave) => call<string>("supplier_save", { input });
// Archives a supplier: no new orders or deliveries.
export const archiveSupplier = (id: string) => call<null>("supplier_archive", { id });
// Purchase orders, newest first; one status only when given (PUR-02).
export const listOrders = (status: OrderStatus | null) => call<OrderList>("po_list", { status });
// One order with the value of each line.
export const getOrder = (id: string) => call<OrderView>("po_get", { id });
// Saves a draft order; names and SKUs come from the catalog.
export const saveOrder = (input: OrderSave) => call<string>("po_save", { input });
// The low-stock products of this supplier, each for its reorder quantity (PUR-02).
export const suggestLines = (supplierId: string) =>
  call<NewOrderLine[]>("po_suggest", { supplierId });
// The order was printed or handed over: it is sent.
export const markSent = (id: string) => call<null>("po_mark_sent", { id });
// Saves the order as an Excel file; a draft is marked sent.
export const exportOrder = (id: string, file: string) => call<null>("po_export", { id, file });
// Cancels an order before anything arrived.
export const cancelOrder = (id: string) => call<null>("po_cancel", { id });
// The receiving form for an order: what is still to come (PUR-03).
export const startReceiving = (orderId: string) => call<GrnStart>("grn_start", { orderId });
// Line values, totals and cost changes of a delivery. Saves nothing.
export const previewDelivery = (input: GrnInput) => call<GrnPreview>("grn_preview", { input });
// Saves the delivery: stock, cost, batches and what the shop owes (PUR-03).
export const saveDelivery = (input: GrnInput) => call<GrnDone>("grn_save", { input });
// Every delivery, newest first.
export const listDeliveries = () => call<GrnRow[]>("grn_list");
// One delivery with its lines.
export const getDelivery = (id: string) => call<Grn>("grn_get", { id });
// Sends goods back to a supplier (PUR-04).
export const sendBack = (input: SupplierReturnInput) =>
  call<SupplierReturnRef>("supplier_return_save", { input });
// Every supplier return, newest first.
export const listReturns = () => call<ReturnRow[]>("supplier_return_list");
// A payment to a supplier; cash can come from the drawer (PUR-05).
export const paySupplier = (input: SupplierPaymentInput) =>
  call<SupplierPaymentDone>("supplier_payment", { input });
// What is owed to each supplier and how late (PUR-05).
export const loadAging = () => call<AgingView>("payables_aging");
// A supplier's statement for the days chosen (PUR-06).
export const loadStatement = (id: string, from: string, to: string) =>
  call<SupplierStatement>("supplier_statement", { id, from, to });
// The statement as an Excel file. Gives the number of rows.
export const exportStatement = (id: string, from: string, to: string, file: string) =>
  call<number>("supplier_statement_export", { id, from, to, file });
// Products by name, SKU or barcode, for order and delivery lines.
export const findProducts = (text: string) =>
  call<SearchHit[]>("product_search", { text, limit: 20 });
