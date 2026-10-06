import { useEffect, useRef, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { PrintResult } from "../../../generated/PrintResult";
import type { SessionView } from "../../../generated/SessionView";
import { shortcuts } from "../../../shared/keyboard/shortcuts";
import { PrintProblem } from "../../../shared/print/PrintProblem";
import { Alert } from "../../../shared/ui/Alert";
import { cancelCart, holdCart, reprintInvoice, reprintLast, resumeCart } from "../api";
import { ButtonBar } from "../components/ButtonBar";
import { CancelDialog } from "../components/CancelDialog";
import { CartTable } from "../components/CartTable";
import { HeldListDialog } from "../components/HeldListDialog";
import { HoldDialog } from "../components/HoldDialog";
import { NoShiftPanel } from "../components/NoShiftPanel";
import { PaymentDialog } from "../components/PaymentDialog";
import { QtyDialog } from "../components/QtyDialog";
import { ReprintDialog } from "../components/ReprintDialog";
import { SaleDoneBar } from "../components/SaleDoneBar";
import { ScanBox } from "../components/ScanBox";
import { TillHeader } from "../components/TillHeader";
import { TotalsPanel } from "../components/TotalsPanel";
import { WeightDialog } from "../components/WeightDialog";
import { useTill } from "../useTill";

type DialogName = "qty" | "hold" | "held" | "cancel" | "payment" | "reprint";

type CheckoutPageProps = {
  session: SessionView;
  // Changes when a shift was opened or closed elsewhere, so the page loads again.
  reloadKey: number;
  // True right after login: a cart with items is "your previous sale" (CHK-19).
  justLoggedIn: boolean;
  onOpenShift: () => void;
  onForceClose: () => void;
  onShiftMenu: () => void;
  onLock: () => void;
  onBackOffice: () => void;
};

// The till (Checkout Screen): scan box on top, cart on the left, totals and Pay on the
// right, buttons below. Everything works by keyboard; every amount comes from Rust.
export function CheckoutPage(props: CheckoutPageProps) {
  const { t } = useTranslation("checkout");
  const till = useTill(props.reloadKey, props.justLoggedIn);
  const [dialog, setDialog] = useState<DialogName | null>(null);
  // A receipt that didn't print, with Retry and the system printer (PAY-09).
  const [printFailed, setPrintFailed] = useState<PrintResult | null>(null);
  // After a reprint that worked: which receipt ("" for the last one).
  const [reprinted, setReprinted] = useState<string | null>(null);
  const scanRef = useRef<HTMLInputElement | null>(null);
  const { view } = till;
  const cart = view?.cart ?? null;
  const lines = cart?.lines ?? [];
  const selectedLine = lines.find((line) => line.line_id === till.selected) ?? null;
  const busy = dialog !== null || till.weighing !== null;

  // The scan box always has the focus when no dialog is open.
  useEffect(() => {
    if (!busy) {
      scanRef.current?.focus();
    }
  });

  const close = () => setDialog(null);
  const open = (name: DialogName, allowed: boolean) => () => {
    if (allowed) {
      setDialog(name);
    }
  };
  const hasLines = lines.length > 0;
  const removeSelected = () => {
    if (selectedLine) {
      void till.remove(selectedLine.line_id);
    }
  };
  const stepQty = (step: 1 | -1) => {
    if (selectedLine && selectedLine.unit_decimals === 0) {
      void till.changeQty(selectedLine.line_id, selectedLine.qty + step * 1000);
    }
  };
  const options = { enableOnFormTags: true, preventDefault: true, enabled: !busy && cart !== null };
  // react-hotkeys-hook keeps the first callback unless told what it depends on.
  const deps = [selectedLine, hasLines, view, busy];
  useHotkeys(shortcuts.checkoutSearch.keys, () => scanRef.current?.focus(), options, deps);
  useHotkeys(shortcuts.checkoutQty.keys, open("qty", selectedLine !== null), options, deps);
  useHotkeys(shortcuts.checkoutHold.keys, open("hold", hasLines), options, deps);
  useHotkeys(shortcuts.checkoutHeld.keys, open("held", (view?.held_count ?? 0) > 0), options, deps);
  useHotkeys(shortcuts.checkoutCancel.keys, open("cancel", hasLines), options, deps);
  useHotkeys(shortcuts.checkoutPay.keys, open("payment", hasLines), options, deps);
  // Del removes the selected line, unless the scan box has text to edit.
  useHotkeys(
    shortcuts.checkoutRemove.keys,
    () => (scanRef.current?.value ? undefined : removeSelected()),
    { ...options, preventDefault: false },
    deps,
  );
  useHotkeys(shortcuts.checkoutShift.keys, props.onShiftMenu, { ...options, enabled: !busy }, deps);
  useHotkeys(shortcuts.lock.keys, props.onLock, { ...options, enabled: !busy }, deps);

  // Shows how a printout went: a short note when it printed, the banner when not.
  const showPrint = (result: PrintResult | null, reprintOf: string | null) => {
    setPrintFailed(result && !result.printed ? result : null);
    setReprinted(result?.printed && reprintOf !== null ? reprintOf : null);
  };
  const reprint = (invoiceNo: string | null) => {
    till.clearProblem();
    (invoiceNo === null ? reprintLast() : reprintInvoice(invoiceNo))
      .then((result) => showPrint(result, invoiceNo ?? ""))
      .catch(till.fail);
  };
  useHotkeys(
    shortcuts.checkoutReprint.keys,
    () => reprint(null),
    { ...options, enabled: !busy },
    deps,
  );

  const afterHeld = (heldCount: number) =>
    till.setView((old) => (old ? { ...old, held_count: heldCount } : old));

  if (till.loadFailed) {
    return <Alert kind="problem">{till.loadFailed}</Alert>;
  }
  if (view === null) {
    return <p className="p-6">{t("loading")}</p>;
  }
  const mine = view.shift?.mine === true;

  return (
    <div
      className="flex flex-1 flex-col"
      onMouseUp={() => (busy ? undefined : scanRef.current?.focus())}
    >
      <TillHeader shift={view.shift} />
      {!mine || cart === null ? (
        <NoShiftPanel
          otherShift={view.shift}
          onOpenShift={props.onOpenShift}
          onForceClose={props.onForceClose}
        />
      ) : (
        <div className="flex flex-1">
          <main className="flex flex-1 flex-col gap-2 p-3">
            <ScanBox
              inputRef={scanRef}
              hits={till.hits}
              setHits={till.setHits}
              currency={view.currency}
              onSubmit={(text) => void till.scan(text)}
              onEmptyEnter={open("payment", hasLines)}
              onPick={(id) => void till.pick(id)}
              onArrow={till.moveSelection}
              onPlusMinus={stepQty}
              onKey={() => {
                till.clearProblem();
                till.setRestored(false);
              }}
            />
            {till.problem ? <Alert kind="problem">{till.problem}</Alert> : null}
            {till.restored ? <Alert kind="info">{t("restored")}</Alert> : null}
            {till.lastSale ? <SaleDoneBar done={till.lastSale} currency={view.currency} /> : null}
            {printFailed ? (
              <PrintProblem
                key={JSON.stringify(printFailed.printable)}
                result={printFailed}
                onGone={() => setPrintFailed(null)}
              />
            ) : null}
            {reprinted !== null ? (
              <Alert kind="done">
                {reprinted === ""
                  ? t("reprint.doneLast")
                  : t("reprint.done", { invoice: reprinted })}
              </Alert>
            ) : null}
            <div className="flex-1 overflow-auto rounded-ui border border-border bg-panel">
              <CartTable
                lines={lines}
                currency={view.currency}
                selected={till.selected}
                flash={till.flash}
                priceChanged={till.priceChanged}
                onSelect={till.setSelected}
              />
            </div>
          </main>
          <TotalsPanel
            totals={cart.totals}
            taxLabel={cart.tax_label}
            taxMode={cart.tax_mode}
            currency={view.currency}
            empty={!hasLines}
            onPay={open("payment", hasLines)}
          />
        </div>
      )}
      <ButtonBar
        hasLines={hasLines}
        hasSelection={selectedLine !== null}
        heldCount={view.held_count}
        canReprintAny={props.session.permissions.includes("receipt_reprint_any")}
        onSearch={() => scanRef.current?.focus()}
        onQty={open("qty", selectedLine !== null)}
        onHold={open("hold", hasLines)}
        onHeld={open("held", view.held_count > 0)}
        onRemove={removeSelected}
        onCancel={open("cancel", hasLines)}
        onReprint={() => reprint(null)}
        onOldReceipt={open("reprint", true)}
        onShift={props.onShiftMenu}
        onLock={props.onLock}
        onBackOffice={props.onBackOffice}
      />
      {dialog === "qty" && selectedLine ? (
        <QtyDialog
          line={selectedLine}
          onClose={close}
          onSave={(qty) => {
            close();
            void till.changeQty(selectedLine.line_id, qty);
          }}
        />
      ) : null}
      {dialog === "hold" ? (
        <HoldDialog
          onClose={close}
          onHold={(note) => {
            close();
            holdCart(note)
              .then((fresh) => {
                till.showCart(fresh);
                afterHeld(view.held_count + 1);
              })
              .catch(till.fail);
          }}
        />
      ) : null}
      {dialog === "held" ? (
        <HeldListDialog
          currency={view.currency}
          onClose={close}
          onResume={(cartId) => {
            close();
            resumeCart(cartId)
              .then((resumed) => {
                till.showCart(resumed.cart);
                till.setPriceChanged(resumed.price_changed);
                afterHeld(Math.max(view.held_count - 1, 0));
              })
              .catch(till.fail);
          }}
        />
      ) : null}
      {dialog === "cancel" ? (
        <CancelDialog
          itemCount={cart?.totals.item_count ?? 0}
          onClose={close}
          onConfirm={() => {
            close();
            cancelCart()
              .then((fresh) => till.showCart(fresh))
              .catch(till.fail);
          }}
        />
      ) : null}
      {dialog === "payment" ? (
        <PaymentDialog
          methods={view.payment_methods}
          currency={view.currency}
          onClose={close}
          onDone={(done) => {
            close();
            till.saleDone(done);
            showPrint(done.receipt, null);
          }}
        />
      ) : null}
      {dialog === "reprint" ? (
        <ReprintDialog
          onClose={close}
          onReprint={(invoiceNo) => {
            close();
            reprint(invoiceNo);
          }}
        />
      ) : null}
      {till.weighing ? (
        <WeightDialog
          product={till.weighing}
          currency={view.currency}
          canType={props.session.permissions.includes("checkout_manual_weight")}
          onClose={() => till.setWeighing(null)}
          onAdded={(fresh) => {
            till.setWeighing(null);
            till.showCart(fresh, fresh.lines.at(-1)?.line_id ?? null);
          }}
        />
      ) : null}
    </div>
  );
}
