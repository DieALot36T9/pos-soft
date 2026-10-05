import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { RoundingAppliesTo } from "../../../generated/RoundingAppliesTo";
import type { RoundingMode } from "../../../generated/RoundingMode";
import { useSave } from "../../../shared/bridge/useSave";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { moneyPreview, saveSettings } from "../api";
import { SectionFrame } from "./SectionFrame";
import type { SectionProps } from "./types";

const digitChoices = ["0", "1", "2", "3"] as const;

// Currency format and cash rounding (SET-05, SET-06). Rust refuses a change of
// decimals after the first sale.
export function MoneySection({ saved, onSaved }: SectionProps) {
  const { t } = useTranslation("settings");
  const { t: tErrors } = useTranslation("errors");
  const [format, setFormat] = useState<CurrencyFormat>(saved.currency);
  const [preview, setPreview] = useState("");
  const [step, setStep] = useState(
    formatMoney(saved.rounding.increment_minor, { ...saved.currency, symbol: "", group_sep: "" }),
  );
  const [mode, setMode] = useState<RoundingMode>(saved.rounding.mode);
  const [appliesTo, setAppliesTo] = useState<RoundingAppliesTo>(saved.rounding.applies_to);
  const [stepError, setStepError] = useState<string | null>(null);
  const save = useSave();

  useEffect(() => {
    moneyPreview(format)
      .then(setPreview)
      .catch(() => setPreview(""));
  }, [format]);

  const submit = () => {
    const parsed = parseMoney(step, format.minor_digits);
    if (!parsed.ok) {
      setStepError(tErrors(parsed.code as "money_not_a_number"));
      return;
    }
    setStepError(null);
    save.run(
      () =>
        saveSettings({
          section: "money",
          value: {
            currency: format,
            rounding: { increment_minor: parsed.minor, mode, applies_to: appliesTo },
          },
        }),
      onSaved,
    );
  };

  return (
    <SectionFrame
      title={t("sections.money")}
      busy={save.busy}
      done={save.done}
      problem={save.problem}
      onSave={submit}
    >
      <div className="grid grid-cols-3 gap-4">
        <Field
          label={t("money.symbol.label")}
          tip={t("money.symbol.tip")}
          value={format.symbol}
          onChange={(symbol) => setFormat({ ...format, symbol })}
          width="short"
        />
        <SelectField
          label={t("money.digits.label")}
          tip={t("money.digits.tip")}
          value={String(format.minor_digits) as (typeof digitChoices)[number]}
          options={digitChoices.map((digits) => ({ value: digits, label: digits }))}
          onChange={(digits) => setFormat({ ...format, minor_digits: Number(digits) })}
          error={save.errorFor("minor_digits")}
        />
        <SelectField
          label={t("money.grouping.label")}
          tip={t("money.grouping.tip")}
          value={format.grouping}
          options={(["indian", "western"] as const).map((value) => ({
            value,
            label: t(`money.grouping.${value}`),
          }))}
          onChange={(grouping) => setFormat({ ...format, grouping })}
        />
      </div>
      <CheckBox
        label={t("money.symbolBefore.label")}
        tip={t("money.symbolBefore.tip")}
        checked={format.symbol_before}
        onChange={(symbol_before) => setFormat({ ...format, symbol_before })}
      />
      <p className="rounded-ui border border-border bg-bg p-3">
        {t("money.preview")} <strong className="tabular-nums">{preview}</strong>
      </p>
      <div className="grid grid-cols-3 gap-4">
        <Field
          label={t("money.roundingStep.label")}
          tip={t("money.roundingStep.tip")}
          value={step}
          onChange={setStep}
          error={stepError ?? save.errorFor("increment_minor")}
        />
        <SelectField
          label={t("money.roundingMode.label")}
          tip={t("money.roundingMode.tip")}
          value={mode}
          options={(["nearest", "down", "up"] as const).map((value) => ({
            value,
            label: t(`money.roundingMode.${value}`),
          }))}
          onChange={setMode}
        />
        <SelectField
          label={t("money.appliesTo.label")}
          tip={t("money.appliesTo.tip")}
          value={appliesTo}
          options={(["cash_only", "all_payments"] as const).map((value) => ({
            value,
            label: t(`money.appliesTo.${value}`),
          }))}
          onChange={setAppliesTo}
        />
      </div>
    </SectionFrame>
  );
}
