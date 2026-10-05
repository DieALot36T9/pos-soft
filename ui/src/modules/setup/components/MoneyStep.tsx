import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { CurrencyChoice } from "../../../generated/CurrencyChoice";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { RoundingAppliesTo } from "../../../generated/RoundingAppliesTo";
import type { RoundingMode } from "../../../generated/RoundingMode";
import { useSave } from "../../../shared/bridge/useSave";
import { formatMoney, parseMoney } from "../../../shared/format/money";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { currencyChoices, currencyFormatFor, moneyPreview, saveMoney } from "../api";
import { StepFrame } from "./StepFrame";
import type { StepProps } from "./types";

const digitChoices = ["0", "1", "2", "3"] as const;

// Step 2: currency format with a live preview, and cash rounding (SET-05, SET-06).
export function MoneyStep({ saved, onSaved, onBack }: StepProps) {
  const { t } = useTranslation("setup");
  const { t: tErrors } = useTranslation("errors");
  const [choices, setChoices] = useState<CurrencyChoice[]>([]);
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
    currencyChoices()
      .then(setChoices)
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    moneyPreview(format)
      .then(setPreview)
      .catch(() => setPreview(""));
  }, [format]);

  const pickCurrency = (code: string) => {
    currencyFormatFor(code)
      .then(setFormat)
      .catch(() => undefined);
  };

  const next = () => {
    const parsed = parseMoney(step, format.minor_digits);
    if (!parsed.ok) {
      setStepError(tErrors(parsed.code as "money_not_a_number"));
      return;
    }
    setStepError(null);
    save.run(
      () =>
        saveMoney({
          currency: format,
          rounding: { increment_minor: parsed.minor, mode, applies_to: appliesTo },
        }),
      onSaved,
    );
  };

  const currencyOptions = choices.map((choice) => ({
    value: choice.code,
    label: `${choice.code} · ${choice.name} (${choice.symbol})`,
  }));

  return (
    <StepFrame
      title={t("money.title")}
      intro={t("money.intro")}
      busy={save.busy}
      problem={save.problem}
      onBack={onBack}
      onNext={next}
    >
      <div className="grid grid-cols-2 gap-4">
        {currencyOptions.length > 0 ? (
          <SelectField
            label={t("money.currency.label")}
            tip={t("money.currency.tip")}
            value={format.code}
            options={currencyOptions}
            onChange={pickCurrency}
          />
        ) : null}
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
          options={[
            { value: "indian", label: t("money.grouping.indian") },
            { value: "western", label: t("money.grouping.western") },
          ]}
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
        {t("money.preview")}{" "}
        <strong className="tabular-nums" data-testid="money-preview">
          {preview}
        </strong>
      </p>
      <p className="text-text-muted">{t("money.lockNote")}</p>
      <h3 className="text-xl font-semibold">{t("money.rounding.heading")}</h3>
      <div className="grid grid-cols-3 gap-4">
        <Field
          label={t("money.rounding.step.label")}
          tip={t("money.rounding.step.tip")}
          value={step}
          onChange={setStep}
          error={stepError ?? save.errorFor("increment_minor")}
        />
        <SelectField
          label={t("money.rounding.mode.label")}
          tip={t("money.rounding.mode.tip")}
          value={mode}
          options={(["nearest", "down", "up"] as const).map((value) => ({
            value,
            label: t(`money.rounding.mode.${value}`),
          }))}
          onChange={setMode}
        />
        <SelectField
          label={t("money.rounding.appliesTo.label")}
          tip={t("money.rounding.appliesTo.tip")}
          value={appliesTo}
          options={(["cash_only", "all_payments"] as const).map((value) => ({
            value,
            label: t(`money.rounding.appliesTo.${value}`),
          }))}
          onChange={setAppliesTo}
        />
      </div>
    </StepFrame>
  );
}
