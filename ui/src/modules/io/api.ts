import type { Choice } from "../../generated/Choice";
import type { ErrorLine } from "../../generated/ErrorLine";
import type { ExportRequest } from "../../generated/ExportRequest";
import type { FileFormat } from "../../generated/FileFormat";
import type { ImportDone } from "../../generated/ImportDone";
import type { ImportEntity } from "../../generated/ImportEntity";
import type { ImportPreview } from "../../generated/ImportPreview";
import type { ImportProgress } from "../../generated/ImportProgress";
import type { ImportRequest } from "../../generated/ImportRequest";
import { call } from "../../shared/bridge";

// Reads the file and checks every row; saves nothing (IO-01 steps 2 to 5).
export const previewImport = (request: ImportRequest) =>
  call<ImportPreview>("import_preview", { request });
// Backs up, then saves every row in one go (IO-01 step 6).
export const runImport = (request: ImportRequest) => call<ImportDone>("import_commit", { request });
// How far the running import is (IO-05).
export const importProgress = () => call<ImportProgress>("import_progress");
// Stops the running import; nothing of it is saved (IO-05).
export const cancelImport = () => call<null>("import_cancel");
// Saves the rows that were left out, with the problem in words (IO-01 step 7).
export const saveErrorFile = (lines: ErrorLine[], file: string) =>
  call<null>("import_errors_save", { lines, file });
// An empty file with the right headers and one example row (IO-01 step 1).
export const saveTemplate = (entity: ImportEntity, format: FileFormat, file: string) =>
  call<null>("template_download", { entity, format, file });
// Writes the export and returns how many rows it holds (IO-03).
export const exportData = (request: ExportRequest) => call<number>("export_entity", { request });
// The price lists prices can go into or come from.
export const priceLists = () => call<Choice[]>("io_price_lists");
