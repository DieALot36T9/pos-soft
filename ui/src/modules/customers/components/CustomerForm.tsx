import { Save } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { Choice } from "../../../generated/Choice";
import type { Customer } from "../../../generated/Customer";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { CustomerDetails } from "../../../generated/CustomerDetails";
import { useSave } from "../../../shared/bridge/useSave";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { saveCustomer } from "../api";

type CustomerFormProps = {
  // The customer being changed, or null to add one.
  customer: Customer | null;
  currency: CurrencyFormat;
  // Tier and credit limit are shown only when they can be set (CustomerManage).
  tiers: Choice[] | null;
  heading: string;
  saveLabel: string;
  saveTip: string;
  doneText: string;
  onSaved: (id: string) => void;
};

const optional = (text: string) => (text.trim() === "" ? null : text);

// The customer's details: a new customer or a change (CUS-01, CUS-02). Rust cleans the
// phone number and refuses one that another customer has.
export function CustomerForm(props: CustomerFormProps) {
  const { t } = useTranslation("customers");
  const { t: tErrors } = useTranslation("errors");
  const plain = (minor: number) =>
    formatMoney(minor, { ...props.currency, symbol: "", group_sep: "", decimal_sep: "." });
  const old = props.customer;
  const [name, setName] = useState(old?.name ?? "");
  const [phone, setPhone] = useState(old?.phone ?? "");
  const [email, setEmail] = useState(old?.email ?? "");
  const [address, setAddress] = useState(old?.address ?? "");
  const [notes, setNotes] = useState(old?.notes ?? "");
  const [tierId, setTierId] = useState(old?.tier_id ?? "");
  const [limit, setLimit] = useState(plain(old?.credit_limit ?? 0));
  const [optOut, setOptOut] = useState(old?.loyalty_opt_out ?? false);
  const [limitProblem, setLimitProblem] = useState<string | null>(null);
  const save = useSave();

  const submit = () => {
    const parsed = parseMoney(limit, props.currency.minor_digits);
    if (!parsed.ok) {
      setLimitProblem(tErrors("money_not_a_number"));
      return;
    }
    setLimitProblem(null);
    const details: CustomerDetails = {
      phone,
      name,
      email: optional(email),
      address: optional(address),
      tier_id: tierId === "" ? null : tierId,
      credit_limit: parsed.minor,
      loyalty_opt_out: optOut,
      notes: optional(notes),
    };
    save.run(
      () => saveCustomer({ id: old?.id ?? null, details }),
      (id) => {
        if (!old) {
          setName("");
          setPhone("");
          setEmail("");
          setAddress("");
          setNotes("");
          setOptOut(false);
        }
        props.onSaved(id);
      },
    );
  };
  const fieldProblem = ["name", "phone", "credit_limit"].some((field) => save.errorFor(field));

  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <h3 className="text-xl font-semibold">{props.heading}</h3>
      <div className="grid grid-cols-2 gap-3">
        <Field
          label={t("fields.name.label")}
          tip={t("fields.name.tip")}
          value={name}
          onChange={setName}
          error={save.errorFor("name")}
        />
        <Field
          label={t("fields.phone.label")}
          tip={t("fields.phone.tip")}
          value={phone}
          onChange={setPhone}
          error={save.errorFor("phone")}
        />
        <Field
          label={t("fields.email.label")}
          tip={t("fields.email.tip")}
          value={email}
          onChange={setEmail}
        />
        <Field
          label={t("fields.address.label")}
          tip={t("fields.address.tip")}
          value={address}
          onChange={setAddress}
        />
        {props.tiers ? (
          <>
            <SelectField
              label={t("fields.tier.label")}
              tip={t("fields.tier.tip")}
              value={tierId}
              options={[
                { value: "", label: t("fields.tier.none") },
                ...props.tiers.map((tier) => ({ value: tier.id, label: tier.name })),
              ]}
              onChange={setTierId}
            />
            <Field
              label={t("fields.limit.label")}
              tip={t("fields.limit.tip")}
              value={limit}
              onChange={setLimit}
              error={limitProblem ?? save.errorFor("credit_limit")}
              width="short"
            />
          </>
        ) : null}
        <Field
          label={t("fields.notes.label")}
          tip={t("fields.notes.tip")}
          value={notes}
          onChange={setNotes}
        />
      </div>
      <CheckBox
        label={t("fields.optOut.label")}
        tip={t("fields.optOut.tip")}
        checked={optOut}
        onChange={setOptOut}
      />
      {save.problem && !fieldProblem ? <Alert kind="problem">{save.problem}</Alert> : null}
      {save.done ? <Alert kind="done">{props.doneText}</Alert> : null}
      <div className="flex justify-end">
        <ActionButton
          colour="green"
          icon={Save}
          label={props.saveLabel}
          tip={props.saveTip}
          onClick={submit}
        />
      </div>
    </section>
  );
}
