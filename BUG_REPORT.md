# Bug Report: Supershop POS

This report details the issues found during the end-to-end user journey simulation of the Supershop POS frontend application.

## 1. Product Editor Validation Silently Fails (No Client-Side Validation)
**Location:** `ui/src/modules/catalog/pages/ProductEditor.tsx` & `ui/src/shared/bridge/useSave.ts`

**Description:**
When a user attempts to save a new product without filling in the mandatory fields (such as the `name`), the form does not display any validation errors in the UI, nor does it display the "Product was saved" success notification. The application relies entirely on backend validation. If the backend (or the mock API) rejects the payload (e.g. throwing an `AppError` for `name_missing`), the error state is not properly mapped to the individual input fields because the error boundary/mapping logic (`errorField` in `fieldError.ts`) fails to unpack the mocked `AppError` object.

**Impact:**
Users are left confused when clicking "Save product" as nothing happens, and no feedback is provided indicating which fields are required or missing.

**Recommendation:**
Implement client-side validation for required fields in the `Draft` state before attempting to submit to the backend. Additionally, review `errorField.ts` to ensure it correctly handles the `AppError` payload structure thrown by both the Rust backend and the mock API.

---

## 2. Checkout Screen: Discount Button Accessibility & State Logic
**Location:** `ui/src/modules/checkout/components/ButtonBar.tsx` & `ui/src/modules/checkout/pages/CheckoutPage.tsx`

**Description:**
The "Sale discount" (Invoice Discount) and "Discount" (Line Discount) buttons have confusing disabled states.
Specifically, the "Sale discount" button is nested within the secondary actions menu ("More") and uses `disabledReason={noItems}`. The "Discount" button uses `disabledReason={noSelection}`.
During rapid scanning or when clicking product tiles, focus state is sometimes lost, leading to `hasSelection` being false even though an item was just added to the cart. This causes the main "Discount" button to appear disabled despite the cart having items.

**Impact:**
Cashiers might be unable to apply line-item discounts if the item isn't actively selected, which is non-intuitive when clicking a product tile to add it to the cart.

**Recommendation:**
Ensure that when a product is added to the cart via the `ProductCatalogPane` touch tiles, the newly added line item is automatically selected by default so that the "Discount" button becomes immediately active.

---

## 3. "Open Shift" Modal Event Leaking
**Location:** `ui/src/modules/checkout/pages/CheckoutPage.tsx`

**Description:**
When the cashier is prompted with the "Open a shift to start selling" modal, pressing the `Enter` key submits the modal. However, due to standard DOM event propagation, this key press can sometimes leak to the underlying `ScanBox` or other focused elements if the modal unmounts faster than the event bubbling cycle completes.

**Impact:**
This can cause unintended searches or form submissions immediately after opening a shift.

**Recommendation:**
Call `event.preventDefault()` and/or `event.stopPropagation()` on the `Enter` keypress handler within the modal to ensure the keystroke is consumed entirely by the modal.
