import { useState } from "react";
import { useTranslation } from "react-i18next";

import { useSave } from "../../../shared/bridge/useSave";
import { Field } from "../../../shared/ui/Field";
import { saveShop } from "../api";
import { StepFrame } from "./StepFrame";
import type { StepProps } from "./types";

// Step 1: the shop's name and what goes at the top and bottom of receipts (SET-04).
export function ShopStep({ saved, onSaved, onBack }: StepProps) {
  const { t } = useTranslation("setup");
  const shop = saved.shop_profile;
  const [name, setName] = useState(shop.name);
  const [address, setAddress] = useState(shop.address_lines[0] ?? "");
  const [phone, setPhone] = useState(shop.phone);
  const [email, setEmail] = useState(shop.email);
  const [taxRegLabel, setTaxRegLabel] = useState(shop.tax_reg_label);
  const [taxRegNo, setTaxRegNo] = useState(shop.tax_reg_no);
  const [footer, setFooter] = useState(shop.receipt_footer[0] ?? "");
  const save = useSave();

  const next = () =>
    save.run(
      () =>
        saveShop({
          ...shop,
          name,
          address_lines: address.trim() ? [address.trim()] : [],
          phone,
          email,
          tax_reg_label: taxRegLabel,
          tax_reg_no: taxRegNo,
          receipt_footer: footer.trim() ? [footer.trim()] : [],
        }),
      onSaved,
    );

  return (
    <StepFrame
      title={t("shop.title")}
      intro={t("shop.intro")}
      busy={save.busy}
      problem={save.problem}
      onBack={onBack}
      onNext={next}
    >
      <Field
        label={t("shop.name.label")}
        tip={t("shop.name.tip")}
        value={name}
        onChange={setName}
        error={save.errorFor("name")}
      />
      <Field
        label={t("shop.address.label")}
        tip={t("shop.address.tip")}
        value={address}
        onChange={setAddress}
      />
      <div className="grid grid-cols-2 gap-4">
        <Field
          label={t("shop.phone.label")}
          tip={t("shop.phone.tip")}
          value={phone}
          onChange={setPhone}
        />
        <Field
          label={t("shop.email.label")}
          tip={t("shop.email.tip")}
          value={email}
          onChange={setEmail}
          error={save.errorFor("email")}
        />
        <Field
          label={t("shop.taxRegLabel.label")}
          tip={t("shop.taxRegLabel.tip")}
          value={taxRegLabel}
          onChange={setTaxRegLabel}
        />
        <Field
          label={t("shop.taxRegNo.label")}
          tip={t("shop.taxRegNo.tip")}
          value={taxRegNo}
          onChange={setTaxRegNo}
        />
      </div>
      <Field
        label={t("shop.footer.label")}
        tip={t("shop.footer.tip")}
        value={footer}
        onChange={setFooter}
      />
    </StepFrame>
  );
}
