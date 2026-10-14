import { HelpCircle, Keyboard, Palette, X } from "lucide-react";
import { useHotkeys } from "react-hotkeys-hook";

import { shortcuts } from "../keyboard/shortcuts";
import { ActionButton } from "./ActionButton";
import { Dialog } from "./Dialog";

type HelpDialogProps = {
  open: boolean;
  onClose: () => void;
};

const COLOUR_MEANINGS = [
  {
    name: "Green",
    swatch: "bg-green text-white border-green",
    soft: "bg-green-soft text-green border-green",
    meaning: "Money coming in · Finish · Save · Connected",
    examples: "Pay (F10), Complete sale, Save form, Cash payment, Open shift, Weight stable",
  },
  {
    name: "Red",
    swatch: "bg-red text-white border-red",
    soft: "bg-red-soft text-red border-red",
    meaning: "Remove · Cancel · Money going out · Problem",
    examples: "Remove line (Del), Cancel sale (F9), Void, Return (Ctrl+R), Cash out, Delete",
  },
  {
    name: "Amber",
    swatch: "bg-amber-fill text-text border-amber",
    soft: "bg-amber-soft text-amber border-amber",
    meaning: "Wait · Paused · Needs attention or Admin approval",
    examples: "Hold sale (F5), Held sales (F6), Low stock, Expiring soon, Customer credit (Baki)",
  },
  {
    name: "Blue",
    swatch: "bg-blue text-white border-blue",
    soft: "bg-blue-soft text-blue border-blue",
    meaning: "Find · Look up · Information · Selected item",
    examples: "Search (F2), Customer (F3), Reprint (Ctrl+P), Reports, Card payment",
  },
  {
    name: "Purple",
    swatch: "bg-purple text-white border-purple",
    soft: "bg-purple-soft text-purple border-purple",
    meaning: "Price changes · Discounts · Loyalty points",
    examples: "Line discount (F7), Sale discount (Shift+F7), Price override (F8), Promotions",
  },
  {
    name: "Teal",
    swatch: "bg-teal text-white border-teal",
    soft: "bg-teal-soft text-teal border-teal",
    meaning: "Mobile wallet payment",
    examples: "bKash, Nagad, or digital wallet tender rows",
  },
  {
    name: "Grey",
    swatch: "bg-grey text-white border-grey",
    soft: "bg-grey-soft text-grey border-grey",
    meaning: "Neutral · Go back · Lock · Settings",
    examples: "Back (Esc), Shift menu (F11), Lock till (F12), Back office",
  },
] as const;

const TILL_SHORTCUTS = [
  { key: "Enter", action: "Submit barcode/search (or open Payment when scan box is empty)" },
  { key: shortcuts.help, action: "Open this Keyboard & Colour Guide" },
  { key: shortcuts.checkoutSearch.label, action: "Focus barcode / product search box" },
  { key: shortcuts.checkoutCustomer.label, action: "Find or attach a customer to the sale" },
  { key: shortcuts.checkoutQty.label, action: "Change quantity of the selected cart line" },
  { key: "+ / -", action: "Increase or decrease selected line quantity by 1" },
  { key: "3*code", action: "Multiply next scanned item (e.g. type 3* then scan)" },
  { key: shortcuts.checkoutHold.label, action: "Put current sale on hold" },
  { key: shortcuts.checkoutHeld.label, action: "View and resume held sales" },
  { key: shortcuts.checkoutDiscount.label, action: "Apply discount to selected line" },
  { key: shortcuts.checkoutInvoiceDiscount.label, action: "Apply discount to the whole sale" },
  { key: shortcuts.checkoutPrice.label, action: "Override unit price on selected line" },
  { key: shortcuts.checkoutRemove.label, action: "Remove selected line from cart" },
  { key: shortcuts.checkoutCancel.label, action: "Cancel and discard the entire sale" },
  { key: shortcuts.checkoutPay.label, action: "Take payment and complete sale" },
  { key: shortcuts.checkoutShift.label, action: "Shift menu (Cash in/out, X report, Close shift)" },
  { key: shortcuts.lock.label, action: "Lock the till immediately" },
  { key: shortcuts.checkoutReprint.label, action: "Reprint the last receipt (marked COPY)" },
  { key: shortcuts.checkoutReturn.label, action: "Return, exchange, or void a past sale" },
] as const;

const GLOBAL_SHORTCUTS = [
  { key: "Ctrl+K", action: "Quick Jump Command Palette (search any screen or action)" },
  { key: shortcuts.save.label, action: "Save current form in Back Office" },
  { key: shortcuts.back.label, action: "Close dialog / drawer or go back" },
  { key: "F1–F4", action: "Select quick cash notes inside the Payment dialog" },
] as const;

const COLOUR_TITLE = "Colour Code (Every colour has one fixed meaning)";
const COLOUR_DESC =
  "Buttons and badges use consistent colours across the Till and Back Office so you can work by sight.";
const BADGE_STYLE_LABEL = "Badge / Alert style";
const TILL_TITLE = "Till & Checkout Shortcuts";
const GLOBAL_TITLE = "Everywhere & Payment Dialog";

export function HelpDialog({ open, onClose }: HelpDialogProps) {
  useHotkeys("escape", onClose, { enableOnFormTags: true, enabled: open });

  if (!open) return null;

  return (
    <Dialog title="Keyboard Shortcuts & Colour Guide (F1)" colour="blue" width="wide">
      <div className="flex flex-col gap-5 max-h-[75vh] overflow-y-auto pr-1">
        {/* Colour Code Legend */}
        <section className="flex flex-col gap-2">
          <div className="flex items-center gap-2 text-base font-bold text-text">
            <Palette aria-hidden="true" className="w-5 h-5 text-blue" />
            <h3>{COLOUR_TITLE}</h3>
          </div>
          <p className="text-xs text-text-muted">{COLOUR_DESC}</p>
          <div className="grid grid-cols-1 gap-2">
            {COLOUR_MEANINGS.map((item) => (
              <div
                key={item.name}
                className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 rounded-ui border border-border bg-bg p-2.5"
              >
                <div className="flex items-center gap-2.5">
                  <span
                    className={`inline-flex min-w-20 justify-center rounded-ui border px-2.5 py-1 text-xs font-bold ${item.swatch}`}
                  >
                    {item.name}
                  </span>
                  <div>
                    <div className="text-sm font-bold text-text">{item.meaning}</div>
                    <div className="text-xs text-text-muted">{item.examples}</div>
                  </div>
                </div>
                <span
                  className={`self-start sm:self-center rounded-ui border px-2 py-0.5 text-xs font-semibold shrink-0 ${item.soft}`}
                >
                  {BADGE_STYLE_LABEL}
                </span>
              </div>
            ))}
          </div>
        </section>

        {/* Till Shortcuts */}
        <section className="flex flex-col gap-2 border-t border-border pt-4">
          <div className="flex items-center gap-2 text-base font-bold text-text">
            <Keyboard aria-hidden="true" className="w-5 h-5 text-green" />
            <h3>{TILL_TITLE}</h3>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
            {TILL_SHORTCUTS.map((item) => (
              <div
                key={item.key}
                className="flex items-center justify-between gap-3 rounded-ui border border-border bg-panel px-3 py-2"
              >
                <span className="text-xs text-text">{item.action}</span>
                <kbd className="rounded border border-border bg-bg px-2 py-0.5 font-mono text-xs font-bold text-text shrink-0">
                  {item.key}
                </kbd>
              </div>
            ))}
          </div>
        </section>

        {/* Global / Back-Office Shortcuts */}
        <section className="flex flex-col gap-2 border-t border-border pt-4">
          <div className="flex items-center gap-2 text-base font-bold text-text">
            <HelpCircle aria-hidden="true" className="w-5 h-5 text-purple" />
            <h3>{GLOBAL_TITLE}</h3>
          </div>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
            {GLOBAL_SHORTCUTS.map((item) => (
              <div
                key={item.key}
                className="flex items-center justify-between gap-3 rounded-ui border border-border bg-panel px-3 py-2"
              >
                <span className="text-xs text-text">{item.action}</span>
                <kbd className="rounded border border-border bg-bg px-2 py-0.5 font-mono text-xs font-bold text-text shrink-0">
                  {item.key}
                </kbd>
              </div>
            ))}
          </div>
        </section>
      </div>

      <div className="flex justify-end border-t border-border pt-3">
        <ActionButton
          colour="grey"
          icon={X}
          label="Close guide"
          tip="Close this help guide and return to your screen."
          shortcut="Esc"
          onClick={onClose}
        />
      </div>
    </Dialog>
  );
}
