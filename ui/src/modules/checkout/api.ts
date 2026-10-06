import type { CartView } from "../../generated/CartView";
import type { CheckoutView } from "../../generated/CheckoutView";
import type { CustomerRow } from "../../generated/CustomerRow";
import type { CustomerSave } from "../../generated/CustomerSave";
import type { HeldCartView } from "../../generated/HeldCartView";
import type { ManualDiscount } from "../../generated/ManualDiscount";
import type { PaymentEntry } from "../../generated/PaymentEntry";
import type { PaymentPreview } from "../../generated/PaymentPreview";
import type { PrintResult } from "../../generated/PrintResult";
import type { ResumedCart } from "../../generated/ResumedCart";
import type { SaleDone } from "../../generated/SaleDone";
import type { SaleHit } from "../../generated/SaleHit";
import type { ScaleView } from "../../generated/ScaleView";
import type { ScanResult } from "../../generated/ScanResult";
import type { WeightInput } from "../../generated/WeightInput";
import { call } from "../../shared/bridge";

// The shift, the cashier's cart (back exactly as saved, CHK-19), held count and payment methods.
export const loadCheckout = () => call<CheckoutView>("checkout_state");
// Whatever was typed or scanned: a barcode, SKU, scale sticker, "3*code" or a name (CHK-01 to CHK-08).
export const scanInput = (input: string) => call<ScanResult>("checkout_scan", { input });
// Products for the list under the scan box (CHK-03).
export const searchProducts = (text: string) => call<SaleHit[]>("checkout_search", { text });
// Adds a product picked from the search list. A weighed product asks for its weight.
export const addProduct = (productId: string, qty: number | null = null) =>
  call<ScanResult>("checkout_add_product", { productId, qty });
// One live scale reading for the weight dialog (CHK-07).
export const readScale = (productId: string) =>
  call<ScaleView>("checkout_read_scale", { productId });
// Adds a weighed item with the weight from the scale or typed in (CHK-06, CHK-07).
export const addWeighed = (productId: string, weight: WeightInput) =>
  call<CartView>("checkout_add_weighed", { productId, weight });
// Changes a line's quantity in thousandths; 0 removes it (CHK-04).
export const setQty = (lineId: string, qty: number) =>
  call<CartView>("checkout_set_qty", { lineId, qty });
// Takes a line out of the cart (CHK-05).
export const removeLine = (lineId: string) => call<CartView>("checkout_remove_line", { lineId });
// Puts the cart aside with an optional note (CHK-09).
export const holdCart = (note: string | null) => call<CartView>("checkout_hold", { note });
// The carts on hold at this till (CHK-10).
export const heldCarts = () => call<HeldCartView[]>("checkout_held_list");
// Brings a held cart back, repriced (CHK-10).
export const resumeCart = (cartId: string) => call<ResumedCart>("checkout_resume", { cartId });
// Throws the whole cart away; it goes into the audit log (CHK-05).
export const cancelCart = () => call<CartView>("checkout_cancel");
// What is due, paid, left and the change for the payments typed so far (PAY-01, PAY-03, PAY-06).
export const previewPayment = (payments: PaymentEntry[]) =>
  call<PaymentPreview>("checkout_payment_preview", { payments });
// Completes the sale in one transaction (PAY-07), then prints the receipt and opens
// the drawer for cash. A printer problem comes back in `receipt`; the sale stays (PAY-09).
// `approval` is an admin's token for credit over the customer's limit (PAY-04).
export const completeSale = (payments: PaymentEntry[], approval: string | null = null) =>
  call<SaleDone>("checkout_complete", { payments, approval });
// A discount on one line, or null to take it off (CHK-12). Above the user's limit Rust
// answers "override_required" and the call is made again with an admin's token.
export const setLineDiscount = (
  lineId: string,
  discount: ManualDiscount | null,
  approval: string | null,
) => call<CartView>("checkout_set_discount", { lineId, discount, approval });
// A discount on the whole cart, or null to take it off (CHK-12). Same limit as above.
export const setInvoiceDiscount = (discount: ManualDiscount | null, approval: string | null) =>
  call<CartView>("checkout_set_invoice_discount", { discount, approval });
// A typed unit price for one line, or null for the normal price (CHK-13).
export const overridePrice = (lineId: string, price: number | null, approval: string | null) =>
  call<CartView>("checkout_price_override", { lineId, price, approval });
// Puts a customer on the sale, or takes them off with null (CHK-11).
export const setCustomer = (customerId: string | null) =>
  call<CartView>("checkout_set_customer", { customerId });
// Sells at another price list, or normal prices with null (CHK-17).
export const setPriceList = (priceListId: string | null, approval: string | null) =>
  call<CartView>("checkout_set_price_list", { priceListId, approval });
// Customers whose phone or name matches what was typed (CUS-02).
export const findCustomers = (text: string) => call<CustomerRow[]>("customer_lookup", { text });
// Adds a customer with only a phone and a name (quick create, CUS-02).
export const quickAddCustomer = (input: CustomerSave) => call<string>("customer_save", { input });
// Reprints this till's last receipt, marked COPY (PAY-10).
export const reprintLast = () => call<PrintResult>("receipt_reprint_last");
// Reprints any receipt by its invoice number, marked COPY. Needs ReceiptReprintAny (PAY-10).
export const reprintInvoice = (invoiceNo: string) =>
  call<PrintResult>("receipt_reprint", { invoiceNo });
