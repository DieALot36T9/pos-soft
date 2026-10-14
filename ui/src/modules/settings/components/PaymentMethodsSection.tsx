import { Plus } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { MethodKind } from "../../../generated/MethodKind";
import type { PaymentMethodRow } from "../../../generated/PaymentMethodRow";
import type { PaymentMethodSave } from "../../../generated/PaymentMethodSave";
import { errorMessage } from "../../../shared/bridge";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { Tip } from "../../../shared/ui/Tip";
import { listPaymentMethods, savePaymentMethod } from "../api";
import { SectionFrame } from "./SectionFrame";

const blank: PaymentMethodSave = {
  id: null,
  name: "",
  kind: "card",
  requires_reference: false,
  active: true,
  sort: 10,
};

export function PaymentMethodsSection() {
  const { t } = useTranslation("settings");
  const { t: tErrors } = useTranslation("errors");
  const [rows, setRows] = useState<PaymentMethodRow[]>([]);
  const [form, setForm] = useState<PaymentMethodSave>(blank);
  const [problem, setProblem] = useState<string | null>(null);
  const [done, setDone] = useState(false);

  const load = () =>
    listPaymentMethods()
      .then(setRows)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps -- load once.
  }, []);

  const save = () => {
    setDone(false);
    savePaymentMethod(form)
      .then(() => {
        setProblem(null);
        setDone(true);
        setForm(blank);
        return load();
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  const kinds: MethodKind[] = ["cash", "card", "wallet", "other"];
  const yesNo = (value: boolean) => (value ? t("payments.yes") : t("payments.no"));

  return (
    <SectionFrame
      title={t("sections.payments")}
      busy={false}
      done={done}
      problem={problem}
      onSave={save}
    >
      <p className="text-text-muted">{t("payments.intro")}</p>
      <table className="w-full border-collapse rounded-ui border border-border">
        <thead className="border-b border-border bg-bg text-left text-sm font-bold">
          <tr>
            <th className="p-2.5">{t("payments.table.name")}</th>
            <th className="p-2.5">{t("payments.table.kind")}</th>
            <th className="p-2.5">{t("payments.table.reference")}</th>
            <th className="p-2.5">{t("payments.table.active")}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.id} className="border-t border-border hover:bg-blue-soft/30">
              <td className="p-2.5">
                <Tip tip={t("payments.editTip", { name: row.name })}>
                  <button
                    type="button"
                    className="font-semibold text-blue underline cursor-pointer"
                    onClick={() => setForm({ ...row })}
                  >
                    {row.name}
                  </button>
                </Tip>
              </td>
              <td className="p-2.5">{t(`payments.kinds.${row.kind}`)}</td>
              <td className="p-2.5">{yesNo(row.requires_reference)}</td>
              <td className="p-2.5 font-medium">{yesNo(row.active)}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <div>
        <ActionButton
          colour="blue"
          icon={Plus}
          label={t("payments.newMethod.label")}
          tip={t("payments.newMethod.tip")}
          onClick={() => setForm(blank)}
        />
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4 border-t border-border pt-4">
        <Field
          label={t("payments.name.label")}
          tip={t("payments.name.tip")}
          value={form.name}
          onChange={(name) => setForm({ ...form, name })}
        />
        <SelectField
          label={t("payments.kind.label")}
          tip={t("payments.kind.tip")}
          value={form.kind}
          options={(kinds.includes(form.kind) ? kinds : [form.kind, ...kinds]).map((kind) => ({
            value: kind,
            label: t(`payments.kinds.${kind}`),
          }))}
          onChange={(kind) => setForm({ ...form, kind })}
        />
      </div>
      <div className="flex flex-wrap gap-6">
        <CheckBox
          label={t("payments.reference.label")}
          tip={t("payments.reference.tip")}
          checked={form.requires_reference}
          onChange={(requires_reference) => setForm({ ...form, requires_reference })}
        />
        <CheckBox
          label={t("payments.active.label")}
          tip={t("payments.active.tip")}
          checked={form.active}
          onChange={(active) => setForm({ ...form, active })}
        />
      </div>
    </SectionFrame>
  );
}
