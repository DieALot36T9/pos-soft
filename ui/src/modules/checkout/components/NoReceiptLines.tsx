import { Plus, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { SaleHit } from "../../../generated/SaleHit";
import { formatMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Tip } from "../../../shared/ui/Tip";

// One product coming back without a receipt, as typed: quantity, a price only when it
// differs from today's, and whether it is damaged.
export type NoReceiptLine = { hit: SaleHit; qty: string; price: string; damaged: boolean };

type NoReceiptLinesProps = {
  hits: SaleHit[];
  lines: NoReceiptLine[];
  currency: CurrencyFormat;
  onPick: (hit: SaleHit) => void;
  onChange: (lines: NoReceiptLine[]) => void;
};

// A return without a receipt (RET-03): products picked from the search, at today's price.
export function NoReceiptLines({ hits, lines, currency, onPick, onChange }: NoReceiptLinesProps) {
  const { t } = useTranslation("checkout");
  const money = (minor: number) => formatMoney(minor, currency);
  const change = (index: number, patch: Partial<NoReceiptLine>) =>
    onChange(lines.map((line, at) => (at === index ? { ...line, ...patch } : line)));
  return (
    <>
      {hits.length > 0 ? (
        <ul className="flex flex-col gap-1">
          {hits.map((hit) => (
            <li key={hit.product_id}>
              <Tip tip={t("returns.pickTip")}>
                <button
                  type="button"
                  onClick={() => onPick(hit)}
                  className="flex min-h-10 w-full items-center gap-2 rounded-ui border border-border px-3 text-left"
                >
                  <Plus aria-hidden="true" className="size-4" />
                  {t("returns.pick", { name: hit.name, price: money(hit.sale_price) })}
                </button>
              </Tip>
            </li>
          ))}
        </ul>
      ) : null}
      {lines.length > 0 ? (
        <table className="w-full">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("returns.item")}</th>
              <th className="p-2">{t("returns.back")}</th>
              <th className="p-2">{t("returns.price")}</th>
              <th className="p-2">{t("returns.damaged")}</th>
              <th className="p-2" />
            </tr>
          </thead>
          <tbody>
            {lines.map((line, index) => (
              <tr key={line.hit.product_id} className="border-b border-border">
                <td className="p-2">{line.hit.name}</td>
                <td className="p-2">
                  <Tip tip={t("returns.qty.tip", { max: "—" })}>
                    <input
                      aria-label={t("returns.qty.label", { name: line.hit.name })}
                      value={line.qty}
                      onChange={(event) => change(index, { qty: event.target.value })}
                      className="min-h-10 w-24 rounded-ui border border-border bg-panel px-3 tabular-nums"
                    />
                  </Tip>
                </td>
                <td className="p-2">
                  <Tip tip={t("returns.unitPrice.tip")}>
                    <input
                      aria-label={t("returns.unitPrice.label", { name: line.hit.name })}
                      value={line.price}
                      placeholder={money(line.hit.sale_price)}
                      onChange={(event) => change(index, { price: event.target.value })}
                      className="min-h-10 w-32 rounded-ui border border-border bg-panel px-3 tabular-nums"
                    />
                  </Tip>
                </td>
                <td className="p-2">
                  <CheckBox
                    label={t("returns.damagedBox.label")}
                    tip={t("returns.damagedBox.tip")}
                    checked={line.damaged}
                    onChange={(ticked) => change(index, { damaged: ticked })}
                  />
                </td>
                <td className="p-2">
                  <ActionButton
                    colour="red"
                    icon={Trash2}
                    label={t("returns.remove.label")}
                    tip={t("returns.remove.tip")}
                    onClick={() => onChange(lines.filter((_, at) => at !== index))}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}
    </>
  );
}
