import { FolderOpen, Layers, Plus, X } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { ProductRow } from "../../../generated/ProductRow";
import { useSave } from "../../../shared/bridge/useSave";
import { formatMoney } from "../../../shared/format/money";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { IconButton } from "../../../shared/ui/IconButton";
import { generateVariants } from "../api";

type VariantsTabProps = {
  parentId: string;
  variants: ProductRow[];
  currency: CurrencyFormat;
  onMade: () => void;
  onOpen: (id: string) => void;
};

type Attribute = { name: string; values: string };

// Attributes and "Make variants" for a parent product, and its variants (CAT-06).
export function VariantsTab({ parentId, variants, currency, onMade, onOpen }: VariantsTabProps) {
  const { t } = useTranslation("catalog");
  const [attributes, setAttributes] = useState<Attribute[]>([{ name: "", values: "" }]);
  const [made, setMade] = useState<number | null>(null);
  const save = useSave();

  const changeAt = (index: number, part: Partial<Attribute>) =>
    setAttributes(attributes.map((each, at) => (at === index ? { ...each, ...part } : each)));

  const generate = () =>
    save.run(
      () =>
        generateVariants(
          parentId,
          attributes.map((each) => ({ name: each.name, values: each.values.split(",") })),
        ),
      (ids) => {
        setMade(ids.length);
        onMade();
      },
    );

  return (
    <div className="flex flex-col gap-3">
      <p className="text-text-muted">{t("variants.intro")}</p>
      {attributes.map((attribute, index) => (
        <div key={index} className="flex items-end gap-3">
          <Field
            label={t("variants.attribute.label")}
            tip={t("variants.attribute.tip")}
            value={attribute.name}
            onChange={(name) => changeAt(index, { name })}
          />
          <Field
            label={t("variants.values.label")}
            tip={t("variants.values.tip")}
            value={attribute.values}
            onChange={(values) => changeAt(index, { values })}
          />
          {attributes.length > 1 ? (
            <IconButton
              icon={X}
              colour="red"
              tip={t("variants.removeAttribute.tip")}
              onClick={() => setAttributes(attributes.filter((_, at) => at !== index))}
            />
          ) : null}
        </div>
      ))}
      <div className="flex gap-3">
        <ActionButton
          colour="grey"
          icon={Plus}
          label={t("variants.addAttribute.label")}
          tip={t("variants.addAttribute.tip")}
          onClick={() => setAttributes([...attributes, { name: "", values: "" }])}
        />
        <ActionButton
          colour="green"
          icon={Layers}
          label={t("variants.generate.label")}
          tip={t("variants.generate.tip")}
          onClick={generate}
        />
      </div>
      {save.problem ? <Alert kind="problem">{save.problem}</Alert> : null}
      {made !== null ? (
        <Alert kind="done">
          {made === 0 ? t("variants.noneMade") : t("variants.made", { count: made })}
        </Alert>
      ) : null}
      <h3 className="text-xl font-semibold">{t("variants.tableHeading")}</h3>
      {variants.length === 0 ? (
        <p>{t("variants.empty")}</p>
      ) : (
        <table className="w-full border-collapse rounded-ui border border-border bg-panel">
          <thead>
            <tr className="border-b border-border text-left">
              <th className="p-2">{t("products.table.name")}</th>
              <th className="p-2">{t("products.table.sku")}</th>
              <th className="p-2 text-right">{t("products.table.price")}</th>
              <th className="p-2">{t("products.table.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {variants.map((variant) => (
              <tr key={variant.id} className="border-b border-border">
                <td className="p-2">{variant.name}</td>
                <td className="p-2">{variant.sku}</td>
                <td className="p-2 text-right tabular-nums">
                  {formatMoney(variant.sale_price, currency)}
                </td>
                <td className="p-2">
                  <IconButton
                    icon={FolderOpen}
                    colour="blue"
                    tip={t("variants.open.tip")}
                    onClick={() => onOpen(variant.id)}
                  />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
