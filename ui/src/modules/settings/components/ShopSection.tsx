import { useState } from "react";
import { useTranslation } from "react-i18next";

import { useSave } from "../../../shared/bridge/useSave";
import { Field } from "../../../shared/ui/Field";
import { saveSettings } from "../api";
import { SectionFrame } from "./SectionFrame";
import type { SectionProps } from "./types";

// The shop's name, address and tax number (SET-04).
export function ShopSection({ saved, onSaved }: SectionProps) {
  const { t } = useTranslation("settings");
  const shop = saved.shop_profile;
  const [name, setName] = useState(shop.name);
  const [address, setAddress] = useState(shop.address_lines[0] ?? "");
  const [phone, setPhone] = useState(shop.phone);
  const [email, setEmail] = useState(shop.email);
  const [taxRegLabel, setTaxRegLabel] = useState(shop.tax_reg_label);
  const [taxRegNo, setTaxRegNo] = useState(shop.tax_reg_no);
  const save = useSave();

  const submit = () =>
    save.run(
      () =>
        saveSettings({
          section: "shop",
          value: {
            ...shop,
            name,
            address_lines: address.trim()
              ? [address.trim(), ...shop.address_lines.slice(1)]
              : shop.address_lines.slice(1),
            phone,
            email,
            tax_reg_label: taxRegLabel,
            tax_reg_no: taxRegNo,
          },
        }),
      onSaved,
    );

  return (
    <SectionFrame
      title={t("sections.shop")}
      busy={save.busy}
      done={save.done}
      problem={save.problem}
      onSave={submit}
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
    </SectionFrame>
  );
}
