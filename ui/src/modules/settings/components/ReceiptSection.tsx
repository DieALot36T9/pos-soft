import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { PaperWidth } from "../../../generated/PaperWidth";
import type { ReceiptSettings } from "../../../generated/ReceiptSettings";
import { useSave } from "../../../shared/bridge/useSave";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { saveSettings } from "../api";
import { SectionFrame } from "./SectionFrame";
import type { SectionProps } from "./types";

type Switch =
  | "show_logo"
  | "show_tax_breakdown"
  | "show_tax_reg_no"
  | "show_invoice_barcode"
  | "show_customer_balance";

const switches: {
  key: Switch;
  text:
    "showLogo" | "showTaxBreakdown" | "showTaxRegNo" | "showInvoiceBarcode" | "showCustomerBalance";
}[] = [
  { key: "show_logo", text: "showLogo" },
  { key: "show_tax_breakdown", text: "showTaxBreakdown" },
  { key: "show_tax_reg_no", text: "showTaxRegNo" },
  { key: "show_invoice_barcode", text: "showInvoiceBarcode" },
  { key: "show_customer_balance", text: "showCustomerBalance" },
];

// What the receipt shows (SET-14).
export function ReceiptSection({ saved, onSaved }: SectionProps) {
  const { t } = useTranslation("settings");
  const [receipt, setReceipt] = useState<ReceiptSettings>(saved.receipt);
  const [header, setHeader] = useState(saved.receipt.header_lines[0] ?? "");
  const [footer, setFooter] = useState(saved.receipt.footer_lines[0] ?? "");
  const save = useSave();

  const submit = () =>
    save.run(
      () =>
        saveSettings({
          section: "receipt",
          value: {
            ...receipt,
            header_lines: header.trim() ? [header.trim()] : [],
            footer_lines: footer.trim() ? [footer.trim()] : [],
          },
        }),
      onSaved,
    );

  return (
    <SectionFrame
      title={t("sections.receipt")}
      busy={save.busy}
      done={save.done}
      problem={save.problem}
      onSave={submit}
    >
      <div className="grid grid-cols-2 gap-4">
        <SelectField
          label={t("receipt.paper.label")}
          tip={t("receipt.paper.tip")}
          value={receipt.paper_width}
          options={(["mm58", "mm80"] as const).map((value) => ({
            value,
            label: t(`receipt.paper.${value}`),
          }))}
          onChange={(paper_width: PaperWidth) => setReceipt({ ...receipt, paper_width })}
        />
        <SelectField
          label={t("receipt.copies.label")}
          tip={t("receipt.copies.tip")}
          value={String(receipt.copies) as "1" | "2" | "3"}
          options={(["1", "2", "3"] as const).map((value) => ({ value, label: value }))}
          onChange={(copies) => setReceipt({ ...receipt, copies: Number(copies) })}
          error={save.errorFor("copies")}
        />
      </div>
      {switches.map(({ key, text }) => (
        <CheckBox
          key={key}
          label={t(`receipt.${text}.label`)}
          tip={t(`receipt.${text}.tip`)}
          checked={receipt[key]}
          onChange={(on) => setReceipt({ ...receipt, [key]: on })}
        />
      ))}
      <Field
        label={t("receipt.header.label")}
        tip={t("receipt.header.tip")}
        value={header}
        onChange={setHeader}
      />
      <Field
        label={t("receipt.footer.label")}
        tip={t("receipt.footer.tip")}
        value={footer}
        onChange={setFooter}
      />
      <p className="text-text-muted">{t("receipt.previewNote")}</p>
    </SectionFrame>
  );
}
