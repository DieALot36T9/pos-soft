import type { CartView } from "../../generated/CartView";
import type { CheckoutView } from "../../generated/CheckoutView";
import type { HeldCartView } from "../../generated/HeldCartView";
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
export const completeSale = (payments: PaymentEntry[]) =>
  call<SaleDone>("checkout_complete", { payments });
// Reprints this till's last receipt, marked COPY (PAY-10).
export const reprintLast = () => call<PrintResult>("receipt_reprint_last");
// Reprints any receipt by its invoice number, marked COPY. Needs ReceiptReprintAny (PAY-10).
export const reprintInvoice = (invoiceNo: string) =>
  call<PrintResult>("receipt_reprint", { invoiceNo });
