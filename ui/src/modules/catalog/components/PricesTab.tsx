import { useTranslation } from "react-i18next";

import type { PriceHistoryView } from "../../../generated/PriceHistoryView";
import type { ProductChoices } from "../../../generated/ProductChoices";
import { formatMoney } from "../../../shared/format/money";
import { formatDateTime } from "../../../shared/format/time";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import type { DraftProps } from "./draft";

type PricesTabProps = DraftProps & {
  choices: ProductChoices;
  history: PriceHistoryView[];
};

// Sale price, cost and tax rate, with the price history under them (CAT-01, CAT-09).
export function PricesTab({ draft, change, errorFor, choices, history }: PricesTabProps) {
  const { t } = useTranslation("catalog");
  const defaultName = choices.tax_rates.find(
    (rate) => rate.id === choices.default_tax_rate_id,
  )?.name;
  const reasonOf = (reason: string | null) =>
    reason === "manual" || reason === "grn" || reason === "import"
      ? t(`prices.reasons.${reason}`)
      : (reason ?? "");
  return (
    <div className="flex flex-col gap-4">
      <div className="grid grid-cols-3 gap-3">
        <Field
          label={t("prices.sale.label")}
          tip={t("prices.sale.tip")}
          value={draft.salePrice}
          onChange={(salePrice) => change({ salePrice })}
          error={errorFor("sale_price")}
        />
        <Field
          label={t("prices.cost.label")}
          tip={t("prices.cost.tip")}
          value={draft.costPrice}
          onChange={(costPrice) => change({ costPrice })}
          error={errorFor("cost_price")}
        />
        <SelectField
          label={t("prices.tax.label")}
          tip={t("prices.tax.tip")}
          value={draft.taxRateId}
          options={[
            {
              value: "",
              label: defaultName
                ? `${t("prices.defaultRate")}: ${defaultName}`
                : t("prices.defaultRate"),
            },
            ...choices.tax_rates.map((rate) => ({ value: rate.id, label: rate.name })),
          ]}
          onChange={(taxRateId) => change({ taxRateId })}
          error={errorFor("tax_rate_id")}
        />
      </div>
      <p className="text-sm text-text-muted">{t("prices.priceNeedsPermission")}</p>
      <h3 className="text-xl font-semibold">{t("prices.historyHeading")}</h3>
      {history.length === 0 ? (
        <p>{t("prices.historyEmpty")}</p>
      ) : (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("prices.history.when")}</th>
              <th className="p-2">{t("prices.history.which")}</th>
              <th className="p-2 text-right">{t("prices.history.old")}</th>
              <th className="p-2 text-right">{t("prices.history.new")}</th>
              <th className="p-2">{t("prices.history.who")}</th>
              <th className="p-2">{t("prices.history.why")}</th>
            </tr>
          </thead>
          <tbody>
            {history.map((row, index) => (
              <tr key={`${row.changed_at}-${index}`} className="border-b border-border">
                <td className="p-2">{formatDateTime(row.changed_at)}</td>
                <td className="p-2">{t(`prices.fields.${row.field}`)}</td>
                <td className="p-2 text-right tabular-nums">
                  {formatMoney(row.old, choices.currency)}
                </td>
                <td className="p-2 text-right tabular-nums">
                  {formatMoney(row.new, choices.currency)}
                </td>
                <td className="p-2">{row.changed_by_name ?? ""}</td>
                <td className="p-2">{reasonOf(row.reason)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
