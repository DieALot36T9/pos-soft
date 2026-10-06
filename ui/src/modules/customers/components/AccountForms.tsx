import { HandCoins, Star } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { CustomerPage } from "../../../generated/CustomerPage";
import { useSave } from "../../../shared/bridge/useSave";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { adjustPoints, takePayment } from "../api";

type FormProps = { page: CustomerPage; onDone: () => void };

// A payment against the customer's debt (CUS-05). Rust refuses more than they owe.
export function DebtPaymentForm({ page, onDone }: FormProps) {
  const { t } = useTranslation("customers");
  const { t: tErrors } = useTranslation("errors");
  const money = (minor: number) => formatMoney(minor, page.currency);
  const [amount, setAmount] = useState("");
  const [methodId, setMethodId] = useState(page.payment_methods[0]?.id ?? "");
  const [reference, setReference] = useState("");
  const [typingProblem, setTypingProblem] = useState<string | null>(null);
  const [left, setLeft] = useState<number | null>(null);
  const save = useSave();
  const method = page.payment_methods.find((each) => each.id === methodId);

  if (page.credit <= 0 && left === null) {
    return (
      <section className="rounded-ui border border-border bg-panel p-4">
        <h3 className="text-xl font-semibold">{t("payment.heading")}</h3>
        <p>{t("payment.nothingOwed")}</p>
      </section>
    );
  }
  const submit = () => {
    const parsed = parseMoney(amount, page.currency.minor_digits);
    if (!parsed.ok) {
      setTypingProblem(tErrors("money_not_a_number"));
      return;
    }
    setTypingProblem(null);
    save.run(
      () =>
        takePayment({
          customer_id: page.customer.id,
          amount: parsed.minor,
          method_id: methodId,
          reference: reference.trim() === "" ? null : reference,
        }),
      (done) => {
        setAmount("");
        setReference("");
        setLeft(done.credit);
        onDone();
      },
    );
  };

  return (
    <section className="flex flex-col gap-3 rounded-ui border border-green bg-panel p-4">
      <h3 className="text-xl font-semibold">{t("payment.heading")}</h3>
      <p className="text-text-muted">{t("payment.intro")}</p>
      <div className="flex flex-wrap items-end gap-3">
        <Field
          label={t("payment.amount.label")}
          tip={t("payment.amount.tip")}
          value={amount}
          onChange={setAmount}
          error={typingProblem ?? save.errorFor("amount")}
          width="short"
        />
        <SelectField
          label={t("payment.method.label")}
          tip={t("payment.method.tip")}
          value={methodId}
          options={page.payment_methods.map((each) => ({ value: each.id, label: each.name }))}
          onChange={setMethodId}
        />
        {method?.requires_reference ? (
          <Field
            label={t("payment.reference.label")}
            tip={t("payment.reference.tip")}
            value={reference}
            onChange={setReference}
          />
        ) : null}
      </div>
      {save.problem && !save.errorFor("amount") ? (
        <Alert kind="problem">{save.problem}</Alert>
      ) : null}
      {save.done && left !== null ? (
        <Alert kind="done">{t("payment.done", { amount: money(Math.max(left, 0)) })}</Alert>
      ) : null}
      <div className="flex justify-end">
        <ActionButton
          colour="green"
          icon={HandCoins}
          label={t("payment.save.label")}
          tip={t("payment.save.tip")}
          onClick={submit}
        />
      </div>
    </section>
  );
}

// An admin's points correction with a reason (CUS-04).
export function PointsAdjustForm({ page, onDone }: FormProps) {
  const { t } = useTranslation("customers");
  const [points, setPoints] = useState("");
  const [reason, setReason] = useState("");
  const [typingProblem, setTypingProblem] = useState<string | null>(null);
  const [balance, setBalance] = useState<number | null>(null);
  const save = useSave();

  const submit = () => {
    const value = Number(points.trim());
    if (!Number.isInteger(value) || value === 0 || points.trim() === "") {
      setTypingProblem(t("adjust.invalid"));
      return;
    }
    setTypingProblem(null);
    save.run(
      () => adjustPoints(page.customer.id, value, reason),
      (now) => {
        setPoints("");
        setReason("");
        setBalance(now);
        onDone();
      },
    );
  };

  return (
    <section className="flex flex-col gap-3 rounded-ui border border-purple bg-panel p-4">
      <h3 className="text-xl font-semibold">{t("adjust.heading")}</h3>
      <p className="text-text-muted">{t("adjust.intro")}</p>
      <div className="flex flex-wrap items-end gap-3">
        <Field
          label={t("adjust.points.label")}
          tip={t("adjust.points.tip")}
          value={points}
          onChange={setPoints}
          error={typingProblem}
          width="short"
        />
        <Field
          label={t("adjust.reason.label")}
          tip={t("adjust.reason.tip")}
          value={reason}
          onChange={setReason}
          error={save.errorFor("reason")}
        />
      </div>
      {save.problem && !save.errorFor("reason") ? (
        <Alert kind="problem">{save.problem}</Alert>
      ) : null}
      {save.done && balance !== null ? (
        <Alert kind="done">{t("adjust.done", { points: balance })}</Alert>
      ) : null}
      <div className="flex justify-end">
        <ActionButton
          colour="purple"
          icon={Star}
          label={t("adjust.save.label")}
          tip={t("adjust.save.tip")}
          onClick={submit}
        />
      </div>
    </section>
  );
}
