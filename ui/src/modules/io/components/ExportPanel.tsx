import { FileOutput } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { Choice } from "../../../generated/Choice";
import type { ExportEntity } from "../../../generated/ExportEntity";
import type { FileFormat } from "../../../generated/FileFormat";
import { errorMessage } from "../../../shared/bridge";
import { pickDataFile } from "../../../shared/bridge/pickers";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { SelectField } from "../../../shared/ui/SelectField";
import { exportData } from "../api";

const ENTITIES: ExportEntity[] = [
  "products",
  "customers",
  "suppliers",
  "stock_levels",
  "price_list_prices",
  "batches",
];

// Exports (IO-03): what, CSV or Excel, then where to save it.
export function ExportPanel({ priceLists }: { priceLists: Choice[] }) {
  const { t } = useTranslation("io");
  const { t: tErrors } = useTranslation("errors");
  const [entity, setEntity] = useState<ExportEntity>("products");
  const [format, setFormat] = useState<FileFormat>("xlsx");
  const [priceListId, setPriceListId] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const save = () => {
    setProblem(null);
    setNotice(null);
    const name = `${entity.replace(/_/g, "-")}.${format}`;
    pickDataFile(t("export.saveTitle"), name, format)
      .then((file) =>
        file === null
          ? null
          : exportData({
              entity,
              format,
              file,
              price_list_id: priceListId === "" ? null : priceListId,
            }).then((count) => setNotice(t("export.saved", { count }))),
      )
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-end gap-3">
        <div className="w-64">
          <SelectField
            label={t("export.entity.label")}
            tip={t("export.entity.tip")}
            value={entity}
            options={ENTITIES.map((each) => ({ value: each, label: t(`entities.${each}`) }))}
            onChange={setEntity}
          />
        </div>
        <div className="w-48">
          <SelectField<FileFormat>
            label={t("export.format.label")}
            tip={t("export.format.tip")}
            value={format}
            options={[
              { value: "xlsx", label: t("formats.xlsx") },
              { value: "csv", label: t("formats.csv") },
            ]}
            onChange={setFormat}
          />
        </div>
        {entity === "price_list_prices" ? (
          <div className="w-64">
            <SelectField
              label={t("options.priceList.label")}
              tip={t("options.priceList.tip")}
              value={priceListId}
              options={[
                { value: "", label: t("options.priceList.none") },
                ...priceLists.map((list) => ({ value: list.id, label: list.name })),
              ]}
              onChange={setPriceListId}
            />
          </div>
        ) : null}
        <ActionButton
          colour="blue"
          icon={FileOutput}
          label={t("export.run.label")}
          tip={t("export.run.tip")}
          onClick={save}
        />
      </div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {notice ? <Alert kind="done">{notice}</Alert> : null}
    </div>
  );
}
