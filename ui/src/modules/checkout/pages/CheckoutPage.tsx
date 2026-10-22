import { useCallback, useEffect, useRef, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { CartView } from "../../../generated/CartView";
import type { PrintResult } from "../../../generated/PrintResult";
import type { SessionView } from "../../../generated/SessionView";
import { useApproval } from "../../../shared/approval/useApproval";
import { shortcuts } from "../../../shared/keyboard/shortcuts";
import { parseQty } from "../../../shared/format/qty";
import { parseMoney } from "../../../shared/format/money";
import { parsePercent } from "../../../shared/format/percent";
import { PrintProblem } from "../../../shared/print/PrintProblem";
import { Alert } from "../../../shared/ui/Alert";
import { HelpDialog } from "../../../shared/ui/HelpDialog";
import {
  cancelCart,
  cashOutExchange,
  holdCart,
  overridePrice,
  reprintInvoice,
  reprintLast,
  resumeCart,
  setCustomer,
  setInvoiceDiscount,
  setLineDiscount,
  setPriceList,
} from "../api";
import { ButtonBar } from "../components/ButtonBar";
import { CancelDialog } from "../components/CancelDialog";
import { CartTable } from "../components/CartTable";
import { CustomerBar } from "../components/CustomerBar";
import { CustomerDialog } from "../components/CustomerDialog";
import { DiscountDialog } from "../components/DiscountDialog";
import { ExchangeBar } from "../components/ExchangeBar";
import { HeldListDialog } from "../components/HeldListDialog";
import { HoldDialog } from "../components/HoldDialog";
import { LineEditDrawer } from "../components/LineEditDrawer";
import { NoShiftPanel } from "../components/NoShiftPanel";
import { PaymentDialog } from "../components/PaymentDialog";
import { PriceDialog } from "../components/PriceDialog";
import { PriceListDialog } from "../components/PriceListDialog";
import { ProductCatalogPane } from "../components/ProductCatalogPane";
import { QtyDialog } from "../components/QtyDialog";
import { RefundDoneBar, type RefundDone } from "../components/RefundDoneBar";
import { ReprintDialog } from "../components/ReprintDialog";
import { ReturnDialog } from "../components/ReturnDialog";
import { SaleDoneBar } from "../components/SaleDoneBar";
import { ScanBox } from "../components/ScanBox";
import { TillHeader } from "../components/TillHeader";
import { TotalsPanel } from "../components/TotalsPanel";
import { WeightDialog } from "../components/WeightDialog";
import { useTill } from "../useTill";

type DialogName =
  | "qty"
  | "hold"
  | "held"
  | "cancel"
  | "payment"
  | "reprint"
  | "customer"
  | "discount"
  | "invoiceDiscount"
  | "price"
  | "priceList"
  | "return";

type CheckoutPageProps = {
  session: SessionView;
  reloadKey: number;
  justLoggedIn: boolean;
  onOpenShift: () => void;
  onForceClose: () => void;
  onShiftMenu: () => void;
  onLock: () => void;
  onBackOffice: () => void;
};

export function CheckoutPage(props: CheckoutPageProps) {
  const { t } = useTranslation("checkout");
  const till = useTill(props.reloadKey, props.justLoggedIn);
  const [dialog, setDialog] = useState<DialogName | null>(null);
  const [lineEditOpen, setLineEditOpen] = useState(false);
  const [helpOpen, setHelpOpen] = useState(false);
  const [printFailed, setPrintFailed] = useState<PrintResult | null>(null);
  const [reprinted, setReprinted] = useState<string | null>(null);
  const [refundDone, setRefundDone] = useState<RefundDone | null>(null);
  const scanRef = useRef<HTMLInputElement | null>(null);
  const { withApproval, dialog: approvalDialog } = useApproval();
  const { view } = till;
  const cart = view?.cart ?? null;
  const lines = cart?.lines ?? [];
  const selectedLine = lines.find((line) => line.line_id === till.selected) ?? null;
  const busy =
    dialog !== null ||
    lineEditOpen ||
    helpOpen ||
    till.weighing !== null ||
    approvalDialog !== null;

  useEffect(() => {
    if (!busy) {
      scanRef.current?.focus();
    }
  });

  const close = () => {
    setDialog(null);
    setLineEditOpen(false);
  };

  const changeCart = (
    work: (approval: string | null) => Promise<CartView>,
    lineId: string | null = null,
  ) => {
    close();
    till.clearProblem();
    withApproval(work)
      .then((fresh) => till.showCart(fresh, lineId))
      .catch(till.fail);
  };

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
  const deps = [selectedLine, hasLines, view, busy];

  useHotkeys("f1", () => setHelpOpen(true), {
    enableOnFormTags: true,
    preventDefault: true,
    enabled: !busy,
  });
  useHotkeys(shortcuts.checkoutSearch.keys, () => scanRef.current?.focus(), options, deps);
  useHotkeys(shortcuts.checkoutQty.keys, open("qty", selectedLine !== null), options, deps);
  useHotkeys(shortcuts.checkoutCustomer.keys, open("customer", true), options, deps);
  useHotkeys(
    shortcuts.checkoutDiscount.keys,
    open("discount", selectedLine !== null),
    options,
    deps,
  );
  useHotkeys(
    shortcuts.checkoutInvoiceDiscount.keys,
    open("invoiceDiscount", hasLines),
    options,
    deps,
  );
  useHotkeys(shortcuts.checkoutPrice.keys, open("price", selectedLine !== null), options, deps);
  useHotkeys(shortcuts.checkoutHold.keys, open("hold", hasLines), options, deps);
  useHotkeys(shortcuts.checkoutHeld.keys, open("held", (view?.held_count ?? 0) > 0), options, deps);
  useHotkeys(shortcuts.checkoutCancel.keys, open("cancel", hasLines), options, deps);
  useHotkeys(shortcuts.checkoutPay.keys, open("payment", hasLines), options, deps);
  useHotkeys(shortcuts.checkoutReturn.keys, open("return", true), options, deps);
  useHotkeys(
    shortcuts.checkoutRemove.keys,
    () => (scanRef.current?.value ? undefined : removeSelected()),
    { ...options, preventDefault: false },
    deps,
  );
  useHotkeys(shortcuts.checkoutShift.keys, props.onShiftMenu, { ...options, enabled: !busy }, deps);
  useHotkeys(shortcuts.lock.keys, props.onLock, { ...options, enabled: !busy }, deps);

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

  const cashOut = () => {
    till.clearProblem();
    cashOutExchange()
      .then((out) => {
        till.showCart(out.cart);
        setRefundDone({
          title: t("refundDone.exchangeCashOut"),
          cashBack: out.cash_back,
          pointsTakenBack: 0,
        });
        showPrint(out.drawer, null);
      })
      .catch(till.fail);
  };

  const afterHeld = (heldCount: number) =>
    till.setView((old) => (old ? { ...old, held_count: heldCount } : old));

  const onSelectLine = useCallback(
    (id: string) => {
      till.setSelected(id);
    },
    [till],
  );

  const onInspectLine = useCallback(
    (id: string) => {
      till.setSelected(id);
      setLineEditOpen(true);
    },
    [till, setLineEditOpen],
  );

  if (till.loadFailed) {
    return <Alert kind="problem">{till.loadFailed}</Alert>;
  }
  if (view === null) {
    return <p className="p-6">{t("loading")}</p>;
  }
  const mine = view.shift?.mine === true;

  return (
    <div
      className="flex flex-1 flex-col h-full bg-bg text-text overflow-hidden"
      onMouseUp={() => (busy ? undefined : scanRef.current?.focus())}
    >
      <TillHeader shift={view.shift} onHelp={() => setHelpOpen(true)} />
      {!mine || cart === null ? (
        <NoShiftPanel
          otherShift={view.shift}
          onOpenShift={props.onOpenShift}
          onForceClose={props.onForceClose}
        />
      ) : (
        <div className="flex flex-1 overflow-hidden">
          <div className="flex-[3] flex flex-col h-full overflow-hidden">
            <ProductCatalogPane
              currency={view.currency}
              onPickProduct={(productId) => void till.pick(productId)}
              scanBoxSlot={
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
                    setRefundDone(null);
                  }}
                />
              }
              actionBarSlot={
                <ButtonBar
                  hasLines={hasLines}
                  hasSelection={selectedLine !== null}
                  hasPriceLists={view.price_lists.length > 0}
                  heldCount={view.held_count}
                  canReprintAny={props.session.permissions.includes("receipt_reprint_any")}
                  onSearch={() => scanRef.current?.focus()}
                  onCustomer={open("customer", cart !== null)}
                  onQty={open("qty", selectedLine !== null)}
                  onDiscount={open("discount", selectedLine !== null)}
                  onInvoiceDiscount={open("invoiceDiscount", hasLines)}
                  onPrice={open("price", selectedLine !== null)}
                  onPriceList={open("priceList", view.price_lists.length > 0)}
                  onHold={open("hold", hasLines)}
                  onHeld={open("held", view.held_count > 0)}
                  onRemove={removeSelected}
                  onCancel={open("cancel", hasLines)}
                  onReprint={() => reprint(null)}
                  onOldReceipt={open("reprint", true)}
                  onReturn={open("return", cart !== null)}
                  onShift={props.onShiftMenu}
                  onLock={props.onLock}
                  onBackOffice={props.onBackOffice}
                />
              }
            />
          </div>

          <div className="flex-[2] min-w-[420px] flex flex-col h-full bg-panel border-l border-border overflow-hidden">
            <div className="p-3 border-b border-border bg-bg space-y-2 shrink-0">
              <CustomerBar cart={cart} currency={view.currency} />
              {cart.exchange ? (
                <ExchangeBar
                  exchange={cart.exchange}
                  currency={view.currency}
                  onCashOut={cashOut}
                />
              ) : null}
              {till.problem ? <Alert kind="problem">{till.problem}</Alert> : null}
              {till.restored ? <Alert kind="info">{t("restored")}</Alert> : null}
              {refundDone ? <RefundDoneBar done={refundDone} currency={view.currency} /> : null}
              {till.lastSale && !refundDone ? (
                <SaleDoneBar done={till.lastSale} currency={view.currency} />
              ) : null}
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
            </div>

            <div className="flex-1 overflow-auto bg-panel p-2">
              <CartTable
                lines={lines}
                currency={view.currency}
                selected={till.selected}
                flash={till.flash}
                priceChanged={till.priceChanged}
                onSelect={onSelectLine}
                onInspectLine={onInspectLine}
              />
            </div>

            <TotalsPanel
              totals={cart.totals}
              taxLabel={cart.tax_label}
              taxMode={cart.tax_mode}
              currency={view.currency}
              empty={!hasLines}
              onPay={open("payment", hasLines)}
            />
          </div>
        </div>
      )}

      <HelpDialog open={helpOpen} onClose={() => setHelpOpen(false)} />

      <LineEditDrawer
        open={lineEditOpen && selectedLine !== null}
        line={selectedLine}
        currency={view.currency}
        onClose={() => setLineEditOpen(false)}
        onApplyQty={(qtyText) => {
          if (!selectedLine) return;
          const parsed = parseQty(qtyText, selectedLine.unit_decimals);
          if (parsed.ok) {
            void till.changeQty(selectedLine.line_id, parsed.milli);
          }
        }}
        onApplyPercentDiscount={(pctText) => {
          if (!selectedLine) return;
          const parsed = parsePercent(pctText);
          if (parsed.ok) {
            changeCart(
              (approval) =>
                setLineDiscount(
                  selectedLine.line_id,
                  { kind: "percent", value: parsed.bps },
                  approval,
                ),
              selectedLine.line_id,
            );
          }
        }}
        onApplyAmountDiscount={(amtText) => {
          if (!selectedLine) return;
          const parsed = parseMoney(amtText, view.currency.minor_digits);
          if (parsed.ok) {
            changeCart(
              (approval) =>
                setLineDiscount(
                  selectedLine.line_id,
                  { kind: "amount", value: parsed.minor },
                  approval,
                ),
              selectedLine.line_id,
            );
          }
        }}
        onApplyPriceOverride={(priceText) => {
          if (!selectedLine) return;
          const parsed = parseMoney(priceText, view.currency.minor_digits);
          if (parsed.ok) {
            changeCart(
              (approval) => overridePrice(selectedLine.line_id, parsed.minor, approval),
              selectedLine.line_id,
            );
          }
        }}
        onRemoveLine={removeSelected}
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
      {dialog === "customer" ? (
        <CustomerDialog
          currency={view.currency}
          hasCustomer={cart?.customer !== null && cart?.customer !== undefined}
          onClose={close}
          onPick={(customerId) => changeCart(() => setCustomer(customerId))}
        />
      ) : null}
      {dialog === "discount" && selectedLine ? (
        <DiscountDialog
          lineName={selectedLine.name}
          current={selectedLine.manual_discount}
          limit={view.max_discount}
          currency={view.currency}
          onClose={close}
          onSave={(discount) =>
            changeCart(
              (approval) => setLineDiscount(selectedLine.line_id, discount, approval),
              selectedLine.line_id,
            )
          }
        />
      ) : null}
      {dialog === "invoiceDiscount" ? (
        <DiscountDialog
          lineName={null}
          current={cart?.invoice_discount ?? null}
          limit={view.max_discount}
          currency={view.currency}
          onClose={close}
          onSave={(discount) => changeCart((approval) => setInvoiceDiscount(discount, approval))}
        />
      ) : null}
      {dialog === "price" && selectedLine ? (
        <PriceDialog
          line={selectedLine}
          currency={view.currency}
          onClose={close}
          onSave={(price) =>
            changeCart(
              (approval) => overridePrice(selectedLine.line_id, price, approval),
              selectedLine.line_id,
            )
          }
        />
      ) : null}
      {dialog === "priceList" ? (
        <PriceListDialog
          lists={view.price_lists}
          current={cart?.price_list?.id ?? null}
          onClose={close}
          onPick={(priceListId) => changeCart((approval) => setPriceList(priceListId, approval))}
        />
      ) : null}
      {approvalDialog}
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
            setRefundDone(null);
            till.saleDone(done);
            showPrint(done.receipt, null);
          }}
        />
      ) : null}
      {dialog === "return" ? (
        <ReturnDialog
          methods={view.payment_methods}
          currency={view.currency}
          cartHasLines={hasLines}
          onClose={close}
          onReturned={(done) => {
            close();
            if (done.cart) {
              till.showCart(done.cart);
            }
            setRefundDone({
              title: t("refundDone.returned", { number: done.return_no }),
              cashBack: done.cash_back,
              pointsTakenBack: done.points_taken_back,
            });
            showPrint(done.receipt, null);
          }}
          onVoided={(done) => {
            close();
            setRefundDone({
              title: t("refundDone.voided", { number: done.invoice_no }),
              cashBack: done.cash_back,
              pointsTakenBack: 0,
            });
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
