import { useTranslation } from "react-i18next";

import type { ReturnSale } from "../../../generated/ReturnSale";
import { formatQty } from "../../../shared/format/qty";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Tip } from "../../../shared/ui/Tip";

type ReceiptLinesProps = {
  sale: ReturnSale;
  // What is typed in "Coming back", by sale line.
  qtys: Record<string, string>;
  damaged: Record<string, boolean>;
  onQty: (saleLineId: string, text: string) => void;
  onDamaged: (saleLineId: string, ticked: boolean) => void;
};

// The receipt's lines: sold, already returned, and how many come back now (RET-01).
// A line with nothing left to return can't be changed.
export function ReceiptLines({ sale, qtys, damaged, onQty, onDamaged }: ReceiptLinesProps) {
  const { t } = useTranslation("checkout");
  return (
    <table className="w-full">
      <thead>
        <tr className="border-b border-border text-left">
          <th className="p-2">{t("returns.item")}</th>
          <th className="p-2 text-right">{t("returns.sold")}</th>
          <th className="p-2 text-right">{t("returns.returned")}</th>
          <th className="p-2">{t("returns.back")}</th>
          <th className="p-2">{t("returns.damaged")}</th>
        </tr>
      </thead>
      <tbody>
        {sale.lines.map((line) => {
          const max = formatQty(line.returnable, line.unit_decimals);
          return (
            <tr key={line.sale_line_id} className="border-b border-border">
              <td className="p-2">{line.name}</td>
              <td className="p-2 text-right tabular-nums">
                {formatQty(line.sold, line.unit_decimals)} {line.unit_symbol}
              </td>
              <td className="p-2 text-right tabular-nums">
                {formatQty(line.returned, line.unit_decimals)}
              </td>
              <td className="p-2">
                <Tip tip={t("returns.qty.tip", { max })}>
                  <input
                    aria-label={t("returns.qty.label", { name: line.name })}
                    value={qtys[line.sale_line_id] ?? ""}
                    disabled={line.returnable === 0 || sale.voided}
                    placeholder={line.returnable > 0 ? `0 – ${max}` : "0"}
                    onChange={(event) => onQty(line.sale_line_id, event.target.value)}
                    className="min-h-10 w-28 rounded-ui border border-border bg-panel px-3 tabular-nums"
                  />
                </Tip>
              </td>
              <td className="p-2">
                <CheckBox
                  label={t("returns.damagedBox.label")}
                  tip={t("returns.damagedBox.tip")}
                  checked={damaged[line.sale_line_id] ?? false}
                  onChange={(ticked) => onDamaged(line.sale_line_id, ticked)}
                />
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
