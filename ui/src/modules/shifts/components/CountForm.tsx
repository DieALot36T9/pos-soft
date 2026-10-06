import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { CountLine } from "../../../generated/CountLine";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import { formatMoney } from "../../../shared/format/money";
import { Tip } from "../../../shared/ui/Tip";
import { countTotal } from "../api";

type CountFormProps = {
  denominations: number[];
  currency: CurrencyFormat;
  onChange: (counts: CountLine[]) => void;
};

// Counting the drawer note by note (SHF-01, SHF-06). Rust adds up the total.
export function CountForm({ denominations, currency, onChange }: CountFormProps) {
  const { t } = useTranslation("shifts");
  const [counts, setCounts] = useState<Record<number, string>>({});
  const [total, setTotal] = useState(0);

  useEffect(() => {
    const lines: CountLine[] = denominations
      .map((denomination) => ({
        denomination,
        count: Number.parseInt(counts[denomination] ?? "", 10),
      }))
      .filter((line) => Number.isInteger(line.count) && line.count > 0);
    onChange(lines);
    countTotal(lines)
      .then(setTotal)
      .catch(() => setTotal(0));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- add up again only when a count changes.
  }, [counts]);

  return (
    <fieldset className="flex flex-col gap-2">
      <legend className="font-semibold">{t("count.title")}</legend>
      <div className="grid grid-cols-2 gap-2">
        {denominations.map((denomination) => {
          const note = formatMoney(denomination, currency);
          return (
            <label key={denomination} className="flex items-center justify-between gap-2">
              <span className="tabular-nums">{t("count.each.label", { note })}</span>
              <Tip tip={t("count.each.tip", { note })}>
                <input
                  inputMode="numeric"
                  aria-label={t("count.each.label", { note })}
                  value={counts[denomination] ?? ""}
                  onChange={(event) =>
                    setCounts((old) => ({
                      ...old,
                      [denomination]: event.target.value.replace(/\D/g, ""),
                    }))
                  }
                  className="min-h-10 w-24 rounded-ui border border-border px-2 text-right tabular-nums"
                />
              </Tip>
            </label>
          );
        })}
      </div>
      <p className="flex justify-between text-xl font-semibold">
        <span>{t("count.total")}</span>
        <span className="tabular-nums">{formatMoney(total, currency)}</span>
      </p>
    </fieldset>
  );
}
