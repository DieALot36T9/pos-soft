import { useState } from "react";
import { useTranslation } from "react-i18next";

import { useSave } from "../../../shared/bridge/useSave";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { saveSettings } from "../api";
import { SectionFrame } from "./SectionFrame";
import type { SectionProps } from "./types";

const wholeNumber = /^\d+$/;

// Loyalty points and selling on credit (SET-17, CUS-04, CUS-05). Rust checks the numbers;
// both are off until the admin switches them on here.
export function LoyaltyCreditSection({ saved, onSaved }: SectionProps) {
  const { t } = useTranslation("settings");
  const { t: tErrors } = useTranslation("errors");
  const plain = (minor: number) =>
    formatMoney(minor, { ...saved.currency, symbol: "", group_sep: "", decimal_sep: "." });
  const [loyaltyOn, setLoyaltyOn] = useState(saved.loyalty.enabled);
  const [spendUnit, setSpendUnit] = useState(plain(saved.loyalty.spend_unit_minor));
  const [pointsPerUnit, setPointsPerUnit] = useState(String(saved.loyalty.points_per_unit));
  const [pointValue, setPointValue] = useState(plain(saved.loyalty.value_per_point_minor));
  const [minRedeem, setMinRedeem] = useState(String(saved.loyalty.min_redeem));
  const [creditOn, setCreditOn] = useState(saved.credit.enabled);
  const [defaultLimit, setDefaultLimit] = useState(plain(saved.credit.default_limit_minor));
  const [typingProblem, setTypingProblem] = useState<string | null>(null);
  const save = useSave();

  const submit = () => {
    const money = (text: string) => parseMoney(text, saved.currency.minor_digits);
    const spend = money(spendUnit);
    const value = money(pointValue);
    const limit = money(defaultLimit);
    if (!spend.ok || !value.ok || !limit.ok) {
      setTypingProblem(tErrors("money_not_a_number"));
      return;
    }
    if (!wholeNumber.test(pointsPerUnit.trim()) || !wholeNumber.test(minRedeem.trim())) {
      setTypingProblem(t("loyalty.numberInvalid"));
      return;
    }
    setTypingProblem(null);
    save.run(
      () =>
        saveSettings({
          section: "loyalty",
          value: {
            enabled: loyaltyOn,
            spend_unit_minor: spend.minor,
            points_per_unit: Number(pointsPerUnit),
            value_per_point_minor: value.minor,
            min_redeem: Number(minRedeem),
          },
        }).then(() =>
          saveSettings({
            section: "credit",
            value: { enabled: creditOn, default_limit_minor: limit.minor },
          }),
        ),
      onSaved,
    );
  };

  return (
    <SectionFrame
      title={t("sections.loyalty")}
      busy={save.busy}
      done={save.done}
      problem={typingProblem ?? save.problem}
      onSave={submit}
    >
      <h3 className="text-xl font-semibold">{t("loyalty.heading")}</h3>
      <CheckBox
        label={t("loyalty.enabled.label")}
        tip={t("loyalty.enabled.tip")}
        checked={loyaltyOn}
        onChange={setLoyaltyOn}
      />
      <div className="grid grid-cols-2 gap-4">
        <Field
          label={t("loyalty.spendUnit.label")}
          tip={t("loyalty.spendUnit.tip")}
          value={spendUnit}
          onChange={setSpendUnit}
          error={save.errorFor("spend_unit_minor")}
          width="short"
        />
        <Field
          label={t("loyalty.pointsPerUnit.label")}
          tip={t("loyalty.pointsPerUnit.tip")}
          value={pointsPerUnit}
          onChange={setPointsPerUnit}
          error={save.errorFor("points_per_unit")}
          width="short"
        />
        <Field
          label={t("loyalty.pointValue.label")}
          tip={t("loyalty.pointValue.tip")}
          value={pointValue}
          onChange={setPointValue}
          error={save.errorFor("value_per_point_minor")}
          width="short"
        />
        <Field
          label={t("loyalty.minRedeem.label")}
          tip={t("loyalty.minRedeem.tip")}
          value={minRedeem}
          onChange={setMinRedeem}
          error={save.errorFor("min_redeem")}
          width="short"
        />
      </div>
      <h3 className="text-xl font-semibold">{t("credit.heading")}</h3>
      <CheckBox
        label={t("credit.enabled.label")}
        tip={t("credit.enabled.tip")}
        checked={creditOn}
        onChange={setCreditOn}
      />
      <Field
        label={t("credit.defaultLimit.label")}
        tip={t("credit.defaultLimit.tip")}
        value={defaultLimit}
        onChange={setDefaultLimit}
        error={save.errorFor("default_limit_minor")}
        width="short"
      />
    </SectionFrame>
  );
}
