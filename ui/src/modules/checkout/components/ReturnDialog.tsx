import { ArrowLeftRight, ArrowLeft, Ban, RotateCcw, Search } from "lucide-react";
import { useEffect, useState, type KeyboardEvent } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { PaymentMethodView } from "../../../generated/PaymentMethodView";
import type { ReturnDone } from "../../../generated/ReturnDone";
import type { ReturnInput } from "../../../generated/ReturnInput";
import type { ReturnLineInput } from "../../../generated/ReturnLineInput";
import type { ReturnPreview } from "../../../generated/ReturnPreview";
import type { ReturnSale } from "../../../generated/ReturnSale";
import type { SaleHit } from "../../../generated/SaleHit";
import type { VoidDone } from "../../../generated/VoidDone";
import { useApproval } from "../../../shared/approval/useApproval";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { parseQty } from "../../../shared/format/qty";
import { formatDateTime } from "../../../shared/format/time";
import { shortcuts } from "../../../shared/keyboard/shortcuts";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Dialog } from "../../../shared/ui/Dialog";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import {
  completeReturn,
  findSaleToReturn,
  previewReturn,
  searchProducts,
  startExchange,
} from "../api";
import { NoReceiptLines, type NoReceiptLine } from "./NoReceiptLines";
import { ReceiptLines } from "./ReceiptLines";
import { RefundSummary } from "./RefundSummary";
import { VoidDialog } from "./VoidDialog";

type ReturnDialogProps = {
  methods: PaymentMethodView[];
  currency: CurrencyFormat;
  // A cart with items can't become an exchange; the button explains why.
  cartHasLines: boolean;
  onReturned: (done: ReturnDone) => void;
  onVoided: (done: VoidDone) => void;
  onClose: () => void;
};

// The kinds a refund may go back by; customer credit only for a sale with a customer.
const REFUND_KINDS = ["cash", "card", "wallet", "other"];

// Return, exchange or void (RET-01 to RET-07). The receipt is scanned or typed, the
// quantities ticked, and Rust works out every refund amount: nothing is added up here.
export function ReturnDialog(props: ReturnDialogProps) {
  const { t } = useTranslation("checkout");
  const { t: tErrors } = useTranslation("errors");
  const { withApproval, dialog: approvalDialog } = useApproval();
  const [withReceipt, setWithReceipt] = useState(true);
  const [invoice, setInvoice] = useState("");
  const [sale, setSale] = useState<ReturnSale | null>(null);
  const [qtys, setQtys] = useState<Record<string, string>>({});
  const [damaged, setDamaged] = useState<Record<string, boolean>>({});
  const [products, setProducts] = useState<NoReceiptLine[]>([]);
  const [hits, setHits] = useState<SaleHit[]>([]);
  const [searchText, setSearchText] = useState("");
  const [reason, setReason] = useState("");
  const [methodId, setMethodId] = useState("");
  const [reference, setReference] = useState("");
  const [preview, setPreview] = useState<ReturnPreview | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [voiding, setVoiding] = useState(false);
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));

  const refundMethods = props.methods.filter(
    (method) =>
      REFUND_KINDS.includes(method.kind) || (method.kind === "credit" && sale?.customer != null),
  );
  const chosenMethod =
    refundMethods.find((method) => method.id === methodId) ?? refundMethods.at(0) ?? null;

  const lines = (): ReturnLineInput[] | null => {
    const list: ReturnLineInput[] = [];
    if (withReceipt) {
      for (const line of sale?.lines ?? []) {
        const text = qtys[line.sale_line_id] ?? "";
        if (text.trim() === "") {
          continue;
        }
        const qty = parseQty(text, line.unit_decimals);
        if (!qty.ok) {
          return null;
        }
        if (qty.milli > 0) {
          list.push({
            sale_line_id: line.sale_line_id,
            product_id: null,
            qty: qty.milli,
            restock: !damaged[line.sale_line_id],
            unit_price: null,
          });
        }
      }
      return list;
    }
    for (const line of products) {
      const qty = parseQty(line.qty, 3);
      const price =
        line.price.trim() === "" ? null : parseMoney(line.price, props.currency.minor_digits);
      if (!qty.ok || (price !== null && !price.ok)) {
        return null;
      }
      if (qty.milli > 0) {
        list.push({
          sale_line_id: null,
          product_id: line.hit.product_id,
          qty: qty.milli,
          restock: !line.damaged,
          unit_price: price?.ok ? price.minor : null,
        });
      }
    }
    return list;
  };
  const input = (): ReturnInput | null => {
    const typed = lines();
    if (typed === null || chosenMethod === null) {
      return null;
    }
    return {
      sale_id: withReceipt ? (sale?.sale_id ?? null) : null,
      lines: typed,
      reason: reason.trim() === "" ? null : reason.trim(),
      refund_method_id: chosenMethod.id,
      reference: reference.trim() === "" ? null : reference.trim(),
    };
  };
  const typed = input();
  const typingProblem = typed === null ? tErrors("qty_not_positive") : null;
  const ready = typed !== null && typed.lines.length > 0 && (sale !== null || !withReceipt);

  // After every change Rust works out the refund again (RET-02, RET-07).
  const key = JSON.stringify(typed);
  useEffect(() => {
    if (typed === null || typed.lines.length === 0 || (withReceipt && sale === null)) {
      return;
    }
    previewReturn(typed)
      .then((answer) => {
        setPreview(answer);
        setProblem(null);
      })
      .catch(fail);
    // eslint-disable-next-line react-hooks/exhaustive-deps -- ask Rust again only when the typing changes.
  }, [key]);

  const find = () => {
    if (invoice.trim() === "") {
      return;
    }
    findSaleToReturn(invoice.trim())
      .then((found) => {
        setSale(found);
        setQtys({});
        setDamaged({});
        setMethodId(found.refund_method_id);
        setProblem(null);
      })
      .catch(fail);
  };
  const search = (text: string) => {
    setSearchText(text);
    if (text.trim().length < 2) {
      setHits([]);
      return;
    }
    searchProducts(text.trim()).then(setHits).catch(fail);
  };
  const finish = (work: (approval: string | null) => Promise<ReturnDone>) => {
    if (!ready) {
      return;
    }
    withApproval(work).then(props.onReturned).catch(fail);
  };
  const refund = () => typed && finish((approval) => completeReturn(typed, approval));
  const exchange = () => typed && finish((approval) => startExchange(typed, approval));
  const asking = approvalDialog !== null || voiding;
  useHotkeys("escape", props.onClose, { enableOnFormTags: true, enabled: !asking });

  const onInvoiceKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      find();
    }
  };
  const money = (minor: number) => formatMoney(minor, props.currency);
  const notReady = ready ? undefined : t("returns.emptyPreview");

  return (
    <Dialog title={t("returns.title")} colour="red" width="wide">
      <div className="flex items-end gap-2">
        {withReceipt ? (
          <>
            <div className="flex-1">
              <Field
                label={t("returns.invoice.label")}
                tip={t("returns.invoice.tip")}
                value={invoice}
                onChange={setInvoice}
                onKeyDown={onInvoiceKey}
                autoFocus
              />
            </div>
            <ActionButton
              colour="blue"
              icon={Search}
              label={t("returns.find.label")}
              tip={t("returns.find.tip")}
              shortcut={shortcuts.confirm.label}
              onClick={find}
            />
          </>
        ) : (
          <div className="flex-1">
            <Field
              label={t("returns.search.label")}
              tip={t("returns.search.tip")}
              value={searchText}
              onChange={search}
              autoFocus
            />
          </div>
        )}
        <ActionButton
          colour="grey"
          icon={withReceipt ? Ban : RotateCcw}
          label={withReceipt ? t("returns.noReceipt.label") : t("returns.withReceipt.label")}
          tip={withReceipt ? t("returns.noReceipt.tip") : t("returns.withReceipt.tip")}
          onClick={() => {
            setWithReceipt(!withReceipt);
            setSale(null);
            setProducts([]);
            setHits([]);
            setPreview(null);
          }}
        />
      </div>
      {withReceipt && sale ? (
        <>
          <p className="font-semibold">
            {t("returns.sale", {
              number: sale.invoice_no,
              date: formatDateTime(sale.completed_at),
            })}
            {sale.customer ? ` · ${t("returns.customer", { name: sale.customer.name })}` : ""}
          </p>
          {sale.voided ? <Alert kind="problem">{t("returns.voided")}</Alert> : null}
          {sale.earlier_returns.length > 0 ? (
            <p className="text-text-muted">
              {t("returns.earlier", { numbers: sale.earlier_returns.join(", ") })}
            </p>
          ) : null}
          <ReceiptLines
            sale={sale}
            qtys={qtys}
            damaged={damaged}
            onQty={(id, text) => setQtys((old) => ({ ...old, [id]: text }))}
            onDamaged={(id, ticked) => setDamaged((old) => ({ ...old, [id]: ticked }))}
          />
        </>
      ) : null}
      {!withReceipt ? (
        <NoReceiptLines
          hits={hits}
          lines={products}
          currency={props.currency}
          onPick={(hit) => {
            setHits([]);
            setSearchText("");
            setProducts((old) =>
              old.some((line) => line.hit.product_id === hit.product_id)
                ? old
                : [...old, { hit, qty: "1", price: "", damaged: false }],
            );
          }}
          onChange={setProducts}
        />
      ) : null}
      {sale !== null || !withReceipt ? (
        <div className="grid grid-cols-2 gap-3">
          <Field
            label={t("returns.reason.label")}
            tip={t("returns.reason.tip")}
            value={reason}
            onChange={setReason}
          />
          <SelectField
            label={t("returns.method.label")}
            tip={t("returns.method.tip")}
            value={chosenMethod?.id ?? ""}
            options={refundMethods.map((method) => ({
              value: method.id,
              label:
                method.kind === "credit"
                  ? t("returns.method.storeCredit", { name: method.name })
                  : method.name,
            }))}
            onChange={setMethodId}
          />
          {chosenMethod?.requires_reference ? (
            <Field
              label={t("returns.reference.label")}
              tip={t("returns.reference.tip")}
              value={reference}
              onChange={setReference}
            />
          ) : null}
        </div>
      ) : null}
      {preview && ready ? <RefundSummary preview={preview} money={money} /> : null}
      {(typingProblem ?? problem) ? <Alert kind="problem">{typingProblem ?? problem}</Alert> : null}
      <div className="flex flex-wrap justify-between gap-2">
        <ActionButton
          colour="grey"
          icon={ArrowLeft}
          label={t("returns.close.label")}
          tip={t("returns.close.tip")}
          shortcut="Esc"
          onClick={props.onClose}
        />
        <div className="flex gap-2">
          {withReceipt && sale && !sale.voided ? (
            <ActionButton
              colour="red"
              icon={Ban}
              label={t("returns.voidSale.label")}
              tip={t("returns.voidSale.tip")}
              disabledReason={sale.void_blocked ? tErrors(voidCode(sale.void_blocked)) : undefined}
              onClick={() => setVoiding(true)}
            />
          ) : null}
          <ActionButton
            colour="blue"
            icon={ArrowLeftRight}
            label={t("returns.exchange.label")}
            tip={t("returns.exchange.tip")}
            disabledReason={props.cartHasLines ? tErrors("cart_in_progress") : notReady}
            onClick={exchange}
          />
          <ActionButton
            colour="red"
            icon={RotateCcw}
            size="till"
            label={t("returns.complete.label")}
            tip={t("returns.complete.tip")}
            disabledReason={notReady}
            onClick={refund}
          />
        </div>
      </div>
      {voiding && sale ? (
        <VoidDialog
          invoiceNo={sale.invoice_no}
          saleId={sale.sale_id}
          onClose={() => setVoiding(false)}
          onVoided={props.onVoided}
        />
      ) : null}
      {approvalDialog}
    </Dialog>
  );
}

// Why a sale can't be voided, as an error code with a message (RET-04).
function voidCode(code: string): "sale_already_voided" | "sale_has_returns" | "sale_shift_closed" {
  if (code === "sale_already_voided" || code === "sale_has_returns") {
    return code;
  }
  return "sale_shift_closed";
}
