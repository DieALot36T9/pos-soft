import { useTranslation } from "react-i18next";

import type { ProductChoices } from "../../../generated/ProductChoices";
import type { ProductKind } from "../../../generated/ProductKind";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { categoryPaths } from "./categoryPaths";
import type { DraftProps } from "./draft";

const kinds: ProductKind[] = ["standard", "kit", "variant_parent"];

type GeneralTabProps = DraftProps & {
  choices: ProductChoices;
  // The kind can be picked only for a new product.
  isNew: boolean;
  // For a variant: "200 ml / Mango".
  variantLabel: string | null;
};

// Name, SKU, kind, category, brand, unit, weighing and notes (CAT-01).
export function GeneralTab({
  draft,
  change,
  errorFor,
  choices,
  isNew,
  variantLabel,
}: GeneralTabProps) {
  const { t } = useTranslation("catalog");
  return (
    <div className="grid grid-cols-2 gap-3">
      {variantLabel ? (
        <p className="col-span-2 text-text-muted">
          {t("general.variantOf", { label: variantLabel })}
        </p>
      ) : null}
      <Field
        label={t("general.name.label")}
        tip={t("general.name.tip")}
        value={draft.name}
        onChange={(name) => change({ name })}
        error={errorFor("name")}
      />
      <Field
        label={t("general.sku.label")}
        tip={t("general.sku.tip")}
        value={draft.sku}
        onChange={(sku) => change({ sku })}
        error={errorFor("sku")}
      />
      {isNew ? (
        <SelectField
          label={t("general.kind.label")}
          tip={t("general.kind.tip")}
          value={draft.kind}
          options={kinds.map((kind) => ({ value: kind, label: t(`kinds.${kind}`) }))}
          onChange={(kind) =>
            change({ kind, trackStock: kind === "standard" ? draft.trackStock : false })
          }
        />
      ) : (
        <div className="flex flex-col gap-1">
          <span className="font-semibold">{t("general.kind.label")}</span>
          <span className="min-h-10 py-2">{t(`kinds.${draft.kind}`)}</span>
        </div>
      )}
      <SelectField
        label={t("general.category.label")}
        tip={t("general.category.tip")}
        value={draft.categoryId}
        options={categoryPaths(choices.categories).map((each) => ({
          value: each.id,
          label: each.path,
        }))}
        onChange={(categoryId) => change({ categoryId })}
        error={errorFor("category_id")}
      />
      <SelectField
        label={t("general.brand.label")}
        tip={t("general.brand.tip")}
        value={draft.brandId}
        options={[
          { value: "", label: t("general.noBrand") },
          ...choices.brands.map((brand) => ({ value: brand.id, label: brand.name })),
        ]}
        onChange={(brandId) => change({ brandId })}
        error={errorFor("brand_id")}
      />
      <SelectField
        label={t("general.unit.label")}
        tip={t("general.unit.tip")}
        value={draft.unitId}
        options={choices.units.map((unit) => ({
          value: unit.id,
          label: `${unit.name} (${unit.symbol})`,
        }))}
        onChange={(unitId) => change({ unitId })}
        error={errorFor("unit_id")}
      />
      <div className="flex items-end pb-2">
        <CheckBox
          label={t("general.weighed.label")}
          tip={t("general.weighed.tip")}
          checked={draft.isWeighed}
          onChange={(isWeighed) => change({ isWeighed })}
        />
      </div>
      <Field
        label={t("general.scaleCode.label")}
        tip={t("general.scaleCode.tip")}
        value={draft.scaleCode}
        onChange={(scaleCode) => change({ scaleCode })}
        error={errorFor("scale_item_code")}
      />
      {errorFor("is_weighed") ? (
        <p className="col-span-2 text-red">{errorFor("is_weighed")}</p>
      ) : null}
      <div className="col-span-2">
        <Field
          label={t("general.description.label")}
          tip={t("general.description.tip")}
          value={draft.description}
          onChange={(description) => change({ description })}
          error={errorFor("description")}
        />
      </div>
    </div>
  );
}
