import { Plus } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { SearchHit } from "../../../generated/SearchHit";
import { Field } from "../../../shared/ui/Field";
import { Tip } from "../../../shared/ui/Tip";
import { findProducts } from "../api";

// Finds a product by name, SKU or barcode and adds it as a line.
export function ProductPicker({ onPick }: { onPick: (hit: SearchHit) => void }) {
  const { t } = useTranslation("purchasing");
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
                    onPick(hit);
                    setText("");
                  }}
                  className="flex min-h-10 items-center gap-1 rounded-ui border border-border px-3"
                >
                  <Plus aria-hidden="true" className="size-4" />
                  {hit.name}
                </button>
              </Tip>
            </li>
          ))}
        </ul>
      ) : null}
    </div>
  );
}
