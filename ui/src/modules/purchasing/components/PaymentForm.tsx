import { HandCoins } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { SupplierPage } from "../../../generated/SupplierPage";
import { useSave } from "../../../shared/bridge/useSave";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { paySupplier } from "../api";

// A payment to the supplier (PUR-05). Cash can come out of the drawer of the user's
// open shift; Rust records it on the shift and opens the drawer.
export function PaymentForm({ page, onPaid }: { page: SupplierPage; onPaid: () => void }) {
  const { t } = useTranslation("purchasing");
  const { t: tErrors } = useTranslation("errors");
  const save = useSave();
  const [amount, setAmount] = useState("");
  const [methodId, setMethodId] = useState(page.payment_methods[0]?.id ?? "");
  const [reference, setReference] = useState("");
  const [fromDrawer, setFromDrawer] = useState(false);
  const [done, setDone] = useState<string | null>(null);
  const method = page.payment_methods.find((each) => each.id === methodId) ?? null;
  const parsed = parseMoney(amount, page.currency.minor_digits);

  const submit = () => {
    if (!parsed.ok || method === null) {
      return;
    }
    save.run(
      () =>
        paySupplier({
          supplier_id: page.supplier.id,
          amount: parsed.minor,
          method_id: method.id,
          reference: reference.trim() === "" ? null : reference.trim(),
          from_drawer: method.kind === "cash" && fromDrawer,
        }),
      (paid) => {
        setDone(t("suppliers.pay.done", { amount: formatMoney(paid.balance, page.currency) }));
        setAmount("");
        setReference("");
        onPaid();
      },
    );
  };
  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <h4 className="text-xl font-semibold">{t("suppliers.pay.heading")}</h4>
      <div className="grid grid-cols-2 gap-3">
        <Field
          label={t("suppliers.pay.amount.label")}
          tip={t("suppliers.pay.amount.tip")}
          value={amount}
          onChange={setAmount}
          error={amount.trim() !== "" && !parsed.ok ? tErrors("money_not_a_number") : null}
        />
        <SelectField
          label={t("suppliers.pay.method.label")}
          tip={t("suppliers.pay.method.tip")}
          value={methodId}
          options={page.payment_methods.map((each) => ({ value: each.id, label: each.name }))}
          onChange={setMethodId}
        />
        <Field
          label={t("suppliers.pay.reference.label")}
          tip={t("suppliers.pay.reference.tip")}
          value={reference}
          onChange={setReference}
        />
        {method?.kind === "cash" ? (
          <div className="flex flex-col gap-1">
            <CheckBox
              label={t("suppliers.pay.fromDrawer.label")}
              tip={t("suppliers.pay.fromDrawer.tip")}
              checked={fromDrawer}
              onChange={setFromDrawer}
            />
            {fromDrawer && !page.has_open_shift ? (
              <p className="text-text-muted">{t("suppliers.pay.noShift")}</p>
            ) : null}
          </div>
        ) : null}
      </div>
      {save.problem ? <Alert kind="problem">{save.problem}</Alert> : null}
      {done && save.done ? <Alert kind="done">{done}</Alert> : null}
      <div>
        <ActionButton
          colour="green"
          icon={HandCoins}
          label={t("suppliers.pay.save.label")}
          tip={t("suppliers.pay.save.tip")}
          onClick={submit}
        />
      </div>
    </section>
  );
}
