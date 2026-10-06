import { useTranslation } from "react-i18next";

import type { ProductChoices } from "../../../generated/ProductChoices";
import { formatQty } from "../../../shared/format/qty";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import type { DraftProps } from "./draft";

type StockTabProps = DraftProps & {
  choices: ProductChoices;
  // Null for a product not saved yet.
  onHand: number | null;
};

// Stock tracking, expiry, the reorder rule (INV-03) and the preferred supplier.
export function StockTab({ draft, change, errorFor, choices, onHand }: StockTabProps) {
  const { t } = useTranslation("catalog");
  const unit = choices.units.find((each) => each.id === draft.unitId);
  const holdsStock = draft.kind === "standard" || draft.kind === "variant";
  return (
    <div className="grid grid-cols-2 gap-3">
      {onHand !== null && unit ? (
        <p className="col-span-2 font-semibold">
          {t("stock.onHand", { qty: formatQty(onHand, unit.decimals), unit: unit.symbol })}
        </p>
      ) : null}
      {holdsStock ? (
        <CheckBox
          label={t("stock.track.label")}
          tip={t("stock.track.tip")}
          checked={draft.trackStock}
          onChange={(trackStock) => change({ trackStock })}
        />
      ) : (
        <p className="col-span-2 text-text-muted">{t("stock.track.tip")}</p>
      )}
      <CheckBox
        label={t("stock.expiry.label")}
        tip={t("stock.expiry.tip")}
        checked={draft.trackExpiry}
        onChange={(trackExpiry) => change({ trackExpiry })}
      />
      {holdsStock && draft.trackStock ? (
        <>
          <Field
            label={t("stock.reorderLevel.label")}
            tip={t("stock.reorderLevel.tip")}
            value={draft.reorderLevel}
            onChange={(reorderLevel) => change({ reorderLevel })}
            error={errorFor("reorder_level") ?? errorFor("reorder")}
          />
          <Field
            label={t("stock.reorderQty.label")}
            tip={t("stock.reorderQty.tip")}
            value={draft.reorderQty}
            onChange={(reorderQty) => change({ reorderQty })}
            error={errorFor("reorder_qty")}
          />
        </>
      ) : null}
      <SelectField
        label={t("stock.supplier.label")}
        tip={t("stock.supplier.tip")}
        value={draft.supplierId}
        options={[
          { value: "", label: t("stock.noSupplier") },
          ...choices.suppliers.map((supplier) => ({ value: supplier.id, label: supplier.name })),
        ]}
        onChange={(supplierId) => change({ supplierId })}
        error={errorFor("preferred_supplier_id")}
      />
    </div>
  );
}
