import { useTranslation } from "react-i18next";

import type { ImportPreview } from "../../../generated/ImportPreview";
import { SelectField } from "../../../shared/ui/SelectField";

type MappingTableProps = {
  preview: ImportPreview;
  mapping: (number | null)[];
  onChange: (mapping: (number | null)[]) => void;
};

// One drop-down per field: which column of the file holds it (IO-01 step 3).
export function MappingTable({ preview, mapping, onChange }: MappingTableProps) {
  const { t } = useTranslation("io");
  const options = [
    { value: "", label: t("mapping.notInFile") },
    ...preview.headers.map((header, index) => ({ value: String(index), label: header })),
  ];
  const choose = (field: number, value: string) => {
    const next = [...mapping];
    next[field] = value === "" ? null : Number(value);
    onChange(next);
  };
  return (
    <section aria-labelledby="mapping-heading" className="flex flex-col gap-2">
      <h3 id="mapping-heading" className="text-xl font-semibold">
        {t("mapping.heading")}
      </h3>
      <p className="text-text-muted">{t("mapping.intro")}</p>
      <div className="grid grid-cols-3 gap-3">
        {preview.fields.map((field, index) => (
          <SelectField
            key={field.key}
            label={field.required ? t("mapping.required", { field: field.header }) : field.header}
            tip={t("mapping.tip")}
            value={
              mapping[index] === null || mapping[index] === undefined ? "" : String(mapping[index])
            }
            options={options}
            onChange={(value) => choose(index, value)}
          />
        ))}
      </div>
    </section>
  );
}
