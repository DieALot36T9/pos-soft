import { useTranslation } from "react-i18next";

import type { ImportCellError } from "../../../generated/ImportCellError";
import type { ImportPreview } from "../../../generated/ImportPreview";
import { problemText } from "./problems";

// The most problems listed on screen; the error file after the import has them all.
const MAX_SHOWN = 200;

// The first rows as they will be saved, then every problem with its row and column
// (IO-01 step 5). Every value comes from Rust.
export function PreviewTables({ preview }: { preview: ImportPreview }) {
  const { t } = useTranslation("io");
  return (
    <section aria-labelledby="preview-heading" className="flex flex-col gap-3">
      <h3 id="preview-heading" className="text-xl font-semibold">
        {t("preview.heading")}
      </h3>
      <p>
        {t("preview.counts", {
          created: preview.counts.created,
          updated: preview.counts.updated,
          skipped: preview.counts.skipped,
          total: preview.total_rows,
        })}
      </p>
      {preview.rows.length > 0 ? (
        <div className="max-h-80 overflow-auto rounded-ui border border-border">
          <table className="w-full text-left text-sm">
            <caption className="p-2 text-left font-semibold">
              {t("preview.firstRows", { count: preview.rows.length })}
            </caption>
            <thead className="bg-panel">
              <tr>
                <th className="p-2">{t("preview.row")}</th>
                <th className="p-2">{t("preview.action")}</th>
                {preview.fields.map((field) => (
                  <th key={field.key} className="p-2">
                    {field.header}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {preview.rows.map((row) => (
                <tr key={row.row} className="border-t border-border">
                  <td className="p-2 tabular-nums">{row.row}</td>
                  <td className={"p-2 " + (row.action === "skip" ? "text-text-muted" : "")}>
                    {t(`preview.actions.${row.action}`)}
                  </td>
                  {row.cells.map((cell, index) => (
                    <td key={preview.fields[index]?.key ?? index} className="p-2">
                      {cell}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}
      <ErrorTable errors={preview.errors} />
    </section>
  );
}

function ErrorTable({ errors }: { errors: ImportCellError[] }) {
  const { t } = useTranslation("io");
  if (errors.length === 0) {
    return <p className="text-green">{t("preview.noErrors")}</p>;
  }
  return (
    <div className="max-h-80 overflow-auto rounded-ui border border-red">
      <table className="w-full text-left text-sm">
        <caption className="p-2 text-left font-semibold text-red">
          {t("preview.errorsHeading", { count: errors.length })}
        </caption>
        <thead className="bg-panel">
          <tr>
            <th className="p-2">{t("preview.row")}</th>
            <th className="p-2">{t("preview.column")}</th>
            <th className="p-2">{t("preview.value")}</th>
            <th className="p-2">{t("preview.problem")}</th>
          </tr>
        </thead>
        <tbody>
          {errors.slice(0, MAX_SHOWN).map((error, index) => (
            <tr key={`${error.row}-${error.column}-${index}`} className="border-t border-border">
              <td className="p-2 tabular-nums">{error.row}</td>
              <td className="p-2">{error.column}</td>
              <td className="p-2">{error.value}</td>
              <td className="p-2">{problemText(error.code, t)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
