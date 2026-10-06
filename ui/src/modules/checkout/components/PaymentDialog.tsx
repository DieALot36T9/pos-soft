import { ArrowLeft, CheckCircle } from "lucide-react";
import { useEffect, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { PaymentEntry } from "../../../generated/PaymentEntry";
import type { PaymentMethodView } from "../../../generated/PaymentMethodView";
import type { PaymentPreview } from "../../../generated/PaymentPreview";
import type { SaleDone } from "../../../generated/SaleDone";
import { useApproval } from "../../../shared/approval/useApproval";
import { errorMessage } from "../../../shared/bridge";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { shortcuts } from "../../../shared/keyboard/shortcuts";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Dialog } from "../../../shared/ui/Dialog";
import { Tip } from "../../../shared/ui/Tip";
import { completeSale, previewPayment } from "../api";
import { PaymentRow } from "./PaymentRow";

type PaymentDialogProps = {
  methods: PaymentMethodView[];
  currency: CurrencyFormat;
  onDone: (done: SaleDone) => void;
  onClose: () => void;
};

// The payment dialog (PAY-01 to PAY-07). The amounts typed go to Rust after every key,
// and Rust answers with what is due, paid, left and the change: nothing is added up here.
export function PaymentDialog({ methods, currency, onDone, onClose }: PaymentDialogProps) {
  const { t } = useTranslation("checkout");
  const { t: tErrors } = useTranslation("errors");
  const [amounts, setAmounts] = useState<Record<string, string>>({});
  const [references, setReferences] = useState<Record<string, string>>({});
  const [cashTyped, setCashTyped] = useState(false);
  const [preview, setPreview] = useState<PaymentPreview | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const cash = methods.find((method) => method.kind === "cash");
  // Credit over the customer's limit needs an admin (PAY-04).
  const { withApproval, dialog: approvalDialog } = useApproval();
  // Amounts as plain numbers for typing: "1377.00", no symbol or separators.
  const plain = (minor: number) =>
    formatMoney(minor, { ...currency, symbol: "", group_sep: "", decimal_sep: "." });
  const money = (minor: number) => formatMoney(minor, currency);

  const entries = (): PaymentEntry[] | null => {
    const list: PaymentEntry[] = [];
    for (const method of methods) {
      const text = amounts[method.id] ?? "";
      if (text.trim() === "") {
        continue;
      }
      const parsed = parseMoney(text, currency.minor_digits);
      if (!parsed.ok) {
        return null;
      }
      list.push({
        method_id: method.id,
        amount: parsed.minor,
        reference: references[method.id] ?? null,
      });
    }
    return list;
  };

  // After every change, Rust works out the amounts. Until the cashier types into the
  // cash row, it follows what is left to pay in cash (PAY-01).
  useEffect(() => {
    const typed = entries();
    if (typed === null) {
      return;
    }
    previewPayment(typed)
      .then((answer) => {
        setPreview(answer);
        setProblem(null);
        if (cash && !cashTyped) {
          const cashAmount = plain(answer.cash_to_pay);
          if ((amounts[cash.id] ?? "") !== cashAmount) {
            setAmounts((old) => ({ ...old, [cash.id]: cashAmount }));
          }
        }
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- ask Rust again only when the typing changes.
  }, [amounts, references]);

  const complete = () => {
    const typed = entries();
    if (typed === null || preview === null || preview.remaining > 0) {
      return;
    }
    withApproval((approval) => completeSale(typed, approval))
      .then(onDone)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  const setCash = (minor: number) => {
    if (cash) {
      setCashTyped(true);
      setAmounts((old) => ({ ...old, [cash.id]: plain(minor) }));
    }
  };
  // An amount that isn't a number is shown at once; Rust is asked again once it is fixed.
  const typingProblem = entries() === null ? tErrors("money_not_a_number") : null;
  const options = { enableOnFormTags: true, preventDefault: true };
  const asking = approvalDialog !== null;
  useHotkeys("enter", complete, { ...options, enabled: !asking }, [preview, amounts, references]);
  useHotkeys("escape", onClose, { enableOnFormTags: true, enabled: !asking });
  useHotkeys(
    "f1,f2,f3,f4",
    (_, handler) => {
      const index = ["f1", "f2", "f3", "f4"].indexOf(handler.keys?.[0] ?? "");
      const amount = preview?.quick_cash[index];
      if (amount !== undefined) {
        setCash(amount);
      }
    },
    options,
    [preview],
  );
  // What the customer's account allows, next to the credit and points rows (PAY-04, PAY-05).
  const noteFor = (kind: string) => {
    if (kind === "credit" && preview?.credit_available != null) {
      return {
        text: t("payment.creditNote", { amount: money(preview.credit_available) }),
        tip: t("payment.creditTip"),
      };
    }
    const offer = preview?.points_offer;
    if (kind === "points" && offer) {
      return {
        text: t("payment.pointsNote", {
          points: offer.max_points,
          amount: money(offer.max_amount),
        }),
        tip: t("payment.pointsTip", { value: money(offer.value_per_point) }),
        use:
          offer.max_points > 0
            ? {
                label: t("payment.pointsUse"),
                tip: t("payment.pointsUseTip"),
                amount: offer.max_amount,
              }
            : undefined,
      };
    }
    return undefined;
  };
  const focusRow = (index: number) => document.getElementById(`pay-row-${index}`)?.focus();

  return (
    <Dialog title={t("payment.title")} colour="green" width="wide">
      <div className="flex justify-between text-2xl font-semibold">
        <span>{t("payment.due")}</span>
        <span className="tabular-nums">{preview ? money(preview.due) : "—"}</span>
      </div>
      {preview && preview.rounding !== 0 ? (
        <p className="text-right text-text-muted">
          {t("payment.rounding")} {money(preview.rounding)}
        </p>
      ) : null}
      <div className="flex flex-col gap-2 border-y border-border py-3">
        {preview && preview.exchange_credit > 0 ? (
          <Tip tip={t("payment.exchangeTip")}>
            <div className="flex justify-between text-lg text-blue">
              <span>{t("payment.exchange")}</span>
              <span className="tabular-nums">{money(preview.exchange_credit)}</span>
            </div>
          </Tip>
        ) : null}
        {preview && preview.exchange_cash_back > 0 ? (
          <div className="flex justify-between text-lg text-red">
            <span>{t("payment.exchangeBack")}</span>
            <span className="tabular-nums">{money(preview.exchange_cash_back)}</span>
          </div>
        ) : null}
        {methods.map((method, index) => (
          <PaymentRow
            key={method.id}
            method={method}
            inputId={`pay-row-${index}`}
            amount={amounts[method.id] ?? ""}
            reference={references[method.id] ?? ""}
            onAmount={(text) => {
              if (method.kind === "cash") {
                setCashTyped(true);
              }
              setAmounts((old) => ({ ...old, [method.id]: text }));
            }}
            onReference={(text) => setReferences((old) => ({ ...old, [method.id]: text }))}
            onArrow={(step) => focusRow(Math.min(Math.max(index + step, 0), methods.length - 1))}
            note={noteFor(method.kind)}
            onUse={(minor) => setAmounts((old) => ({ ...old, [method.id]: plain(minor) }))}
          />
        ))}
        {preview && preview.quick_cash.length > 0 ? (
          <div className="flex items-center gap-2">
            <span className="text-text-muted">{t("payment.quick")}</span>
            {preview.quick_cash.map((amount, index) => (
              <Tip key={amount} tip={t("payment.quickTip", { amount: money(amount) })}>
                <button
                  type="button"
                  onClick={() => setCash(amount)}
                  className="min-h-10 rounded-ui border border-green bg-green-soft px-3 font-semibold text-green tabular-nums"
                >
                  {money(amount)} <kbd className="text-sm">{shortcuts.quickCash[index]}</kbd>
                </button>
              </Tip>
            ))}
          </div>
        ) : null}
      </div>
      {preview ? (
        <div className="flex items-end justify-between gap-4">
          <span className="text-xl">
            {t("payment.paid")} <b className="tabular-nums">{money(preview.paid)}</b>
          </span>
          <span className="text-xl">
            {t("payment.remaining")} <b className="tabular-nums">{money(preview.remaining)}</b>
          </span>
          <span aria-label={t("payment.change")} className="text-right text-green">
            <span className="block text-xl font-semibold">{t("payment.change")}</span>
            <span className="text-[56px] leading-tight font-semibold tabular-nums">
              {money(preview.change)}
            </span>
          </span>
        </div>
      ) : null}
      {(typingProblem ?? problem) ? <Alert kind="problem">{typingProblem ?? problem}</Alert> : null}
      <div className="flex justify-between">
        <ActionButton
          colour="grey"
          icon={ArrowLeft}
          label={t("payment.back.label")}
          tip={t("payment.back.tip")}
          shortcut="Esc"
          onClick={onClose}
        />
        <ActionButton
          colour="green"
          icon={CheckCircle}
          size="till"
          label={t("payment.complete.label")}
          tip={t("payment.complete.tip")}
          shortcut={shortcuts.confirm.label}
          disabledReason={
            preview && preview.remaining === 0 ? undefined : t("payment.complete.notYet")
          }
          onClick={complete}
        />
      </div>
      {approvalDialog}
    </Dialog>
  );
}
