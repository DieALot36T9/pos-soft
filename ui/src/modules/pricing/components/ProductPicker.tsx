import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { SearchHit } from "../../../generated/SearchHit";
import { formatMoney } from "../../../shared/format/money";
import { Field } from "../../../shared/ui/Field";
import { Tip } from "../../../shared/ui/Tip";
import { findProducts } from "../api";

type ProductPickerProps = {
  currency: CurrencyFormat;
  chosen: SearchHit | null;
  onChoose: (hit: SearchHit) => void;
};

// Finds one product by name, SKU or barcode, for a price list or a promotion.
export function ProductPicker({ currency, chosen, onChoose }: ProductPickerProps) {
  const { t } = useTranslation("pricing");
  const [text, setText] = useState("");
  const [found, setFound] = useState<SearchHit[]>([]);
  const searching = text.trim() !== "";

  useEffect(() => {
    if (!searching) {
      return;
    }
    findProducts(text)
      .then(setFound)
      .catch(() => setFound([]));
    // eslint-disable-next-line react-hooks/exhaustive-deps -- search again only when the text changes.
  }, [text]);
  const hits = searching ? found : [];

  return (
    <div className="flex flex-col gap-2">
      <Field label={t("picker.label")} tip={t("picker.tip")} value={text} onChange={setText} />
      {searching && hits.length === 0 ? (
        <p className="text-text-muted">{t("picker.none")}</p>
      ) : null}
      {hits.length > 0 ? (
        <ul aria-label={t("picker.label")} className="flex flex-wrap gap-2">
          {hits.map((hit) => (
            <li key={hit.product_id}>
              <Tip tip={t("picker.pick", { name: hit.name })}>
                <button
                  type="button"
                  onClick={() => {
                    onChoose(hit);
                    setText("");
                  }}
                  className="min-h-10 rounded-ui border border-border px-3"
                >
                  {hit.name}
                </button>
              </Tip>
            </li>
          ))}
        </ul>
      ) : null}
      {chosen ? (
        <p className="font-semibold">
          {t("picker.chosen", {
            name: chosen.name,
            price: formatMoney(chosen.sale_price, currency),
          })}
        </p>
      ) : null}
    </div>
  );
}
