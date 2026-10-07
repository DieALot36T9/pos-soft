import { Ban, FileDown, FileUp, ListChecks, Save, Upload } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import type { Choice } from "../../../generated/Choice";
import type { FileFormat } from "../../../generated/FileFormat";
import type { ImportDone } from "../../../generated/ImportDone";
import type { ImportEntity } from "../../../generated/ImportEntity";
import type { ImportPreview } from "../../../generated/ImportPreview";
import type { ImportProgress } from "../../../generated/ImportProgress";
import type { ImportRequest } from "../../../generated/ImportRequest";
import type { MatchBy } from "../../../generated/MatchBy";
import { errorMessage } from "../../../shared/bridge";
import { pickDataFile, pickImportFile } from "../../../shared/bridge/pickers";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { SelectField } from "../../../shared/ui/SelectField";
import {
  cancelImport,
  importProgress,
  previewImport,
  runImport,
  saveErrorFile,
  saveTemplate,
} from "../api";
import { MappingTable } from "./MappingTable";
import { PreviewTables } from "./PreviewTables";
import { problemText } from "./problems";

const ENTITIES: ImportEntity[] = [
  "products",
  "customers",
  "suppliers",
  "opening_stock",
  "price_list_prices",
  "barcodes",
];

// How often the progress bar asks Rust how far the import is.
const PROGRESS_EVERY_MS = 250;

// The import wizard (IO-01, IO-02, IO-05): what, template, file, sheet, columns, match
// rule, preview with every problem, import with progress and cancel, summary.
export function ImportWizard({ priceLists }: { priceLists: Choice[] }) {
  const { t } = useTranslation("io");
  const { t: tErrors } = useTranslation("errors");
  const [entity, setEntity] = useState<ImportEntity>("products");
  const [file, setFile] = useState<string | null>(null);
  const [sheet, setSheet] = useState<string | null>(null);
  const [mapping, setMapping] = useState<(number | null)[] | null>(null);
  const [matchBy, setMatchBy] = useState<MatchBy>("sku");
  const [createMissing, setCreateMissing] = useState(false);
  const [skipErrors, setSkipErrors] = useState(false);
  const [priceListId, setPriceListId] = useState("");
  const [preview, setPreview] = useState<ImportPreview | null>(null);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [done, setDone] = useState<ImportDone | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);
  const fail = (error: unknown) => setProblem(errorMessage(error, tErrors));

  useEffect(
    () => () => {
      if (timer.current !== null) {
        clearInterval(timer.current);
      }
    },
    [],
  );

  const request = (overrides: Partial<ImportRequest> = {}): ImportRequest => ({
    entity,
    file: file ?? "",
    sheet,
    mapping,
    match_by: matchBy,
    create_missing: createMissing,
    skip_errors: skipErrors,
    price_list_id: priceListId === "" ? null : priceListId,
    ...overrides,
  });

  const check = (wanted: ImportRequest) => {
    setProblem(null);
    setNotice(null);
    setDone(null);
    previewImport(wanted)
      .then((found) => {
        setPreview(found);
        setMapping(found.mapping);
      })
      .catch((error: unknown) => {
        setPreview(null);
        fail(error);
      });
  };

  const changeEntity = (next: ImportEntity) => {
    setEntity(next);
    setMapping(null);
    setPreview(null);
    setDone(null);
  };

  const chooseFile = () => {
    pickImportFile(t("file.pickTitle"))
      .then((picked) => {
        if (picked === null) {
          return;
        }
        setFile(picked);
        setSheet(null);
        setMapping(null);
        check(request({ file: picked, sheet: null, mapping: null }));
      })
      .catch(fail);
  };

  const template = (format: FileFormat) => {
    setProblem(null);
    const name = `${entity.replace(/_/g, "-")}-template.${format}`;
    pickDataFile(t("template.saveTitle"), name, format)
      .then((target) =>
        target === null
          ? null
          : saveTemplate(entity, format, target).then(() => setNotice(t("template.saved"))),
      )
      .catch(fail);
  };

  const stopPolling = () => {
    if (timer.current !== null) {
      clearInterval(timer.current);
      timer.current = null;
    }
  };

  const start = () => {
    setProblem(null);
    setNotice(null);
    setProgress({ done: 0, total: preview?.total_rows ?? 0, running: true });
    timer.current = setInterval(() => {
      importProgress()
        .then(setProgress)
        .catch(() => undefined);
    }, PROGRESS_EVERY_MS);
    runImport(request())
      .then((finished) => {
        setDone(finished);
        setPreview(null);
      })
      .catch((error: unknown) => {
        const cancelled =
          typeof error === "object" &&
          error !== null &&
          "code" in error &&
          error.code === "import_cancelled";
        if (cancelled) {
          setNotice(t("run.cancelled"));
        } else {
          fail(error);
        }
      })
      .finally(() => {
        stopPolling();
        setProgress(null);
      });
  };

  const saveErrors = () => {
    if (done === null) {
      return;
    }
    const lines = done.errors.map((error) => ({
      row: error.row,
      column: error.column,
      value: error.value,
      problem: problemText(error.code, t),
    }));
    pickDataFile(t("done.errorFileTitle"), "import-errors.csv", "csv")
      .then((target) =>
        target === null
          ? null
          : saveErrorFile(lines, target).then(() => setNotice(t("done.errorFileSaved"))),
      )
      .catch(fail);
  };

  const startAgain = () => {
    setDone(null);
    setFile(null);
    setPreview(null);
    setMapping(null);
    setNotice(null);
  };

  const findsProducts = entity === "opening_stock" || entity === "price_list_prices";
  const blocked =
    preview === null || file === null
      ? t("run.needFile")
      : preview.errors.length > 0 && !skipErrors
        ? t("run.hasErrors")
        : undefined;

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-end gap-3">
        <div className="w-64">
          <SelectField
            label={t("entity.label")}
            tip={t("entity.tip")}
            value={entity}
            options={ENTITIES.map((each) => ({ value: each, label: t(`entities.${each}`) }))}
            onChange={changeEntity}
          />
        </div>
        <ActionButton
          colour="blue"
          icon={FileDown}
          label={t("template.csv.label")}
          tip={t("template.csv.tip")}
          onClick={() => template("csv")}
        />
        <ActionButton
          colour="blue"
          icon={FileDown}
          label={t("template.xlsx.label")}
          tip={t("template.xlsx.tip")}
          onClick={() => template("xlsx")}
        />
      </div>

      <div className="flex flex-wrap items-center gap-3">
        <ActionButton
          colour="blue"
          icon={FileUp}
          label={t("file.choose.label")}
          tip={t("file.choose.tip")}
          onClick={chooseFile}
        />
        <p>{file === null ? t("file.none") : t("file.chosen", { file })}</p>
      </div>

      {preview !== null && preview.sheets.length > 0 ? (
        <div className="w-64">
          <SelectField
            label={t("sheet.label")}
            tip={t("sheet.tip")}
            value={sheet ?? preview.sheets[0] ?? ""}
            options={preview.sheets.map((name) => ({ value: name, label: name }))}
            onChange={(name) => {
              setSheet(name);
              setMapping(null);
              check(request({ sheet: name, mapping: null }));
            }}
          />
        </div>
      ) : null}

      {preview !== null && mapping !== null ? (
        <MappingTable preview={preview} mapping={mapping} onChange={setMapping} />
      ) : null}

      <div className="flex flex-wrap items-end gap-4">
        {entity === "products" || findsProducts ? (
          <div className="w-56">
            <SelectField<MatchBy>
              label={t("options.matchBy.label")}
              tip={t("options.matchBy.tip")}
              value={matchBy}
              options={[
                { value: "sku", label: t("options.matchBy.sku") },
                { value: "barcode", label: t("options.matchBy.barcode") },
              ]}
              onChange={setMatchBy}
            />
          </div>
        ) : null}
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
        {entity === "products" ? (
          <CheckBox
            label={t("options.createMissing.label")}
            tip={t("options.createMissing.tip")}
            checked={createMissing}
            onChange={setCreateMissing}
          />
        ) : null}
        <ActionButton
          colour="blue"
          icon={ListChecks}
          label={t("check.label")}
          tip={t("check.tip")}
          disabledReason={file === null ? t("run.needFile") : undefined}
          onClick={() => check(request())}
        />
      </div>

      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {notice ? <Alert kind="done">{notice}</Alert> : null}

      {preview !== null ? <PreviewTables preview={preview} /> : null}

      {preview !== null && progress === null ? (
        <div className="flex flex-wrap items-center gap-4">
          <CheckBox
            label={t("options.skipErrors.label")}
            tip={t("options.skipErrors.tip")}
            checked={skipErrors}
            onChange={setSkipErrors}
          />
          <ActionButton
            colour="green"
            icon={Upload}
            label={t("run.label")}
            tip={t("run.tip")}
            disabledReason={blocked}
            onClick={start}
          />
        </div>
      ) : null}

      {progress !== null ? (
        <div className="flex items-center gap-4">
          <div className="flex flex-1 flex-col gap-1">
            <label htmlFor="import-progress">
              {t("run.progress", { done: progress.done, total: progress.total })}
            </label>
            <progress
              id="import-progress"
              className="h-4 w-full"
              value={progress.done}
              max={Math.max(progress.total, 1)}
            />
          </div>
          <ActionButton
            colour="red"
            icon={Ban}
            label={t("run.cancel.label")}
            tip={t("run.cancel.tip")}
            onClick={() => void cancelImport().catch(fail)}
          />
        </div>
      ) : null}

      {done !== null ? (
        <section aria-labelledby="done-heading" className="flex flex-col gap-2">
          <h3 id="done-heading" className="text-xl font-semibold">
            {t("done.heading")}
          </h3>
          <Alert kind="done">
            {t("done.counts", {
              created: done.counts.created,
              updated: done.counts.updated,
              skipped: done.counts.skipped,
            })}
          </Alert>
          <p className="text-text-muted">{t("done.backup", { file: done.backup_file })}</p>
          <div className="flex gap-2">
            {done.errors.length > 0 ? (
              <ActionButton
                colour="blue"
                icon={Save}
                label={t("done.errorFile.label")}
                tip={t("done.errorFile.tip")}
                onClick={saveErrors}
              />
            ) : null}
            <ActionButton
              colour="grey"
              icon={FileUp}
              label={t("done.again.label")}
              tip={t("done.again.tip")}
              onClick={startAgain}
            />
          </div>
        </section>
      ) : null}
    </div>
  );
}
