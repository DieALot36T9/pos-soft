import { Save } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { Supplier } from "../../../generated/Supplier";
import { useSave } from "../../../shared/bridge/useSave";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { saveSupplier } from "../api";

type SupplierFormProps = {
  // None: a new supplier.
  supplier: Supplier | null;
  heading: string;
  saveLabel: string;
  saveTip: string;
  onSaved: (id: string) => void;
};

type FormKey = "name" | "contact" | "phone" | "email" | "address" | "taxNumber" | "terms" | "notes";

const blank = (text: string) => (text.trim() === "" ? null : text.trim());

// A supplier's details (PUR-01). Rust checks every field; the name is required.
export function SupplierForm({
  supplier,
  heading,
  saveLabel,
  saveTip,
  onSaved,
}: SupplierFormProps) {
  const { t } = useTranslation("purchasing");
  const save = useSave();
  const [name, setName] = useState(supplier?.name ?? "");
  const [contact, setContact] = useState(supplier?.contact_person ?? "");
  const [phone, setPhone] = useState(supplier?.phone ?? "");
  const [email, setEmail] = useState(supplier?.email ?? "");
  const [address, setAddress] = useState(supplier?.address ?? "");
  const [taxNumber, setTaxNumber] = useState(supplier?.tax_number ?? "");
  const [terms, setTerms] = useState(String(supplier?.payment_terms_days ?? 0));
  const [notes, setNotes] = useState(supplier?.notes ?? "");
  const termsOk = /^\d{1,4}$/.test(terms.trim());

  const submit = () => {
    if (!termsOk) {
      return;
    }
    save.run(
      () =>
        saveSupplier({
          id: supplier?.id ?? null,
          input: {
            name,
            contact_person: blank(contact),
            phone: blank(phone),
            email: blank(email),
            address: blank(address),
            tax_number: blank(taxNumber),
            payment_terms_days: Number(terms.trim()),
            notes: blank(notes),
          },
        }),
      (id) => {
        if (supplier === null) {
          setName("");
          setContact("");
          setPhone("");
          setEmail("");
          setAddress("");
          setTaxNumber("");
          setTerms("0");
          setNotes("");
        }
        onSaved(id);
      },
    );
  };
  const f = (key: FormKey) => ({
    label: t(`suppliers.form.${key}.label`),
    tip: t(`suppliers.form.${key}.tip`),
  });
  return (
    <section className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
      <h3 className="text-xl font-semibold">{heading}</h3>
      <div className="grid grid-cols-2 gap-3">
        <Field {...f("name")} value={name} onChange={setName} error={save.errorFor("name")} />
        <Field {...f("contact")} value={contact} onChange={setContact} />
        <Field {...f("phone")} value={phone} onChange={setPhone} />
        <Field {...f("email")} value={email} onChange={setEmail} />
        <Field {...f("address")} value={address} onChange={setAddress} />
        <Field {...f("taxNumber")} value={taxNumber} onChange={setTaxNumber} />
        <Field
          {...f("terms")}
          value={terms}
          onChange={setTerms}
          width="short"
          error={termsOk ? null : t("suppliers.form.termsInvalid")}
        />
        <Field {...f("notes")} value={notes} onChange={setNotes} />
      </div>
      {save.problem ? <Alert kind="problem">{save.problem}</Alert> : null}
      {save.done ? <Alert kind="done">{t("common.saved")}</Alert> : null}
      <div>
        <ActionButton colour="green" icon={Save} label={saveLabel} tip={saveTip} onClick={submit} />
      </div>
    </section>
  );
}
