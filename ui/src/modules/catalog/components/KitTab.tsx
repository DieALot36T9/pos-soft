import { Plus, Save, Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { KitPart } from "../../../generated/KitPart";
import type { KitPartInput } from "../../../generated/KitPartInput";
import type { SearchHit } from "../../../generated/SearchHit";
import { errorMessage } from "../../../shared/bridge";
import { useSave } from "../../../shared/bridge/useSave";
import { formatQty, parseQty } from "../../../shared/format/qty";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { IconButton } from "../../../shared/ui/IconButton";
import { saveKit, searchProducts } from "../api";

type KitTabProps = {
  kitId: string;
  parts: KitPart[];
  onSaved: () => void;
};

// One part as typed: the quantity is text until it is saved.
type Row = { productId: string; name: string; sku: string; unit: string; qty: string };

// The products inside a kit and how many of each (CAT-07). Quantities are typed with up to
// 3 decimals; Rust checks they are above zero and that the kit has no loops.
export function KitTab({ kitId, parts, onSaved }: KitTabProps) {
  const { t } = useTranslation("catalog");
  const { t: tErrors } = useTranslation("errors");
  const [rows, setRows] = useState<Row[]>(
    parts.map((part) => ({
      productId: part.product_id,
      name: part.name,
      sku: part.sku,
      unit: part.unit_symbol,
      qty: formatQty(part.qty, 3).replace(/\.?0+$/, ""),
    })),
  );
  const [text, setText] = useState("");
  const [hits, setHits] = useState<SearchHit[]>([]);
  const [problem, setProblem] = useState<string | null>(null);
  const save = useSave();

  const find = (typed: string) => {
    setText(typed);
    if (typed.trim() === "") {
      setHits([]);
      return;
    }
    searchProducts(typed, 8)
      .then(setHits)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };

  const pick = (hit: SearchHit) => {
    if (!rows.some((row) => row.productId === hit.product_id)) {
      setRows([
        ...rows,
        {
          productId: hit.product_id,
          name: hit.name,
          sku: hit.sku,
          unit: hit.unit_symbol,
          qty: "1",
        },
      ]);
    }
    setText("");
    setHits([]);
  };

  const submit = () => {
    const typedParts: KitPartInput[] = [];
    for (const row of rows) {
      const qty = parseQty(row.qty, 3);
      if (!qty.ok) {
        setProblem(tErrors(qty.code === "qty_empty" ? "qty_empty" : "qty_invalid"));
        return;
      }
      typedParts.push({ product_id: row.productId, qty: qty.milli });
    }
    setProblem(null);
    save.run(() => saveKit(kitId, typedParts), onSaved);
  };

  return (
    <div className="flex flex-col gap-3">
      <p className="text-text-muted">{t("kit.intro")}</p>
      <Field label={t("kit.find.label")} tip={t("kit.find.tip")} value={text} onChange={find} />
      {hits.length > 0 ? (
        <ul className="flex flex-col gap-1 rounded-ui border border-border bg-panel p-2">
          {hits.map((hit) => (
            <li key={hit.product_id} className="flex items-center gap-3">
              <IconButton
                icon={Plus}
                colour="blue"
                tip={t("kit.pick.tip")}
                onClick={() => pick(hit)}
              />
              <span>{t("kit.hit", { name: hit.name, sku: hit.sku })}</span>
            </li>
          ))}
        </ul>
      ) : null}
      {rows.length === 0 ? (
        <p>{t("kit.empty")}</p>
      ) : (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("kit.table.name")}</th>
              <th className="p-2">{t("kit.table.sku")}</th>
              <th className="p-2">{t("kit.table.qty")}</th>
              <th className="p-2">{t("kit.table.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {rows.map((row) => (
              <tr key={row.productId} className="border-b border-border">
                <td className="p-2">{row.name}</td>
                <td className="p-2">{row.sku}</td>
                <td className="p-2">
                  <Field
                    label={`${t("kit.qty.label")} (${row.unit})`}
                    tip={t("kit.qty.tip")}
                    value={row.qty}
                    width="short"
                    onChange={(qty) =>
                      setRows(
                        rows.map((each) =>
                          each.productId === row.productId ? { ...each, qty } : each,
                        ),
                      )
                    }
                  />
                </td>
                <td className="p-2">
                  <IconButton
                    icon={Trash2}
                    colour="red"
                    tip={t("kit.remove.tip")}
                    onClick={() => setRows(rows.filter((each) => each.productId !== row.productId))}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {save.problem ? <Alert kind="problem">{save.problem}</Alert> : null}
      {save.done ? <Alert kind="done">{t("kit.saved")}</Alert> : null}
      <div className="flex justify-end">
        <ActionButton
          colour="green"
          icon={Save}
          label={t("kit.save.label")}
          tip={t("kit.save.tip")}
          onClick={submit}
        />
      </div>
    </div>
  );
}
