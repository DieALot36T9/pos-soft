import type { AppError } from "../../../generated/AppError";
import type { Choice } from "../../../generated/Choice";
import type { CurrencyFormat } from "../../../generated/CurrencyFormat";
import type { ExportRequest } from "../../../generated/ExportRequest";
import type { ImportCellError } from "../../../generated/ImportCellError";
import type { ImportDone } from "../../../generated/ImportDone";
import type { ImportEntity } from "../../../generated/ImportEntity";
import type { ImportField } from "../../../generated/ImportField";
import type { ImportPreview } from "../../../generated/ImportPreview";
import type { ImportProgress } from "../../../generated/ImportProgress";
import type { ImportRequest } from "../../../generated/ImportRequest";
import type { LabelItem } from "../../../generated/LabelItem";
import type { LabelPick } from "../../../generated/LabelPick";
import type { LabelPicked } from "../../../generated/LabelPicked";
import type { LabelPicture } from "../../../generated/LabelPicture";
import type { LabelRequest } from "../../../generated/LabelRequest";
import type { LabelsPrinted } from "../../../generated/LabelsPrinted";
import { fakeCatalog } from "./catalog";
import { requirePermission } from "./people";
import { fakePreviewPrint } from "./printer";
import { testReceiptPicture } from "./testReceipt";

// A pretend Import / Export screen and label printing for the fake bridge. Rust reads
// the real files and checks every cell (Module - io); this stand-in answers with a
// fixed demo file of four product rows, one of them with a problem.

type Args = Record<string, unknown>;

let currency: () => CurrencyFormat = () => {
  throw new Error("The fake import isn't connected to the fake shop.");
};

// shop.ts calls this once, so labels use the shop's money format.
export function connectIoToShop(fromShop: () => CurrencyFormat) {
  currency = fromShop;
}

const field = (key: string, header: string, required = false): ImportField => ({
  key,
  header,
  required,
});

const FIELDS: Record<ImportEntity, ImportField[]> = {
  products: [
    field("sku", "SKU"),
    field("name", "Name", true),
    field("barcodes", "Barcodes"),
    field("sale_price", "Sale price", true),
    field("cost_price", "Cost price"),
  ],
  customers: [field("phone", "Phone", true), field("name", "Name", true)],
  suppliers: [field("name", "Name", true), field("phone", "Phone")],
  opening_stock: [field("product", "SKU", true), field("qty", "Quantity", true)],
  price_list_prices: [field("product", "SKU", true), field("price", "Price", true)],
  barcodes: [field("product", "SKU", true), field("barcode", "Barcode", true)],
};

// The demo file: headers people often use, and a typing mistake on row 4.
const HEADERS = ["Item code", "Product name", "Barcode", "MRP", "Cost"];
const ROWS = [
  ["P000101", "Milk 1L", "8901234567890", "90.00", "75.00"],
  ["P000102", "Rice 5kg", "", "450.00", "400.00"],
  ["P000001", "Lux Soap", "", "60.00", "45.00"],
  ["P000103", "Sugar 1kg", "", "12,5O", "110.00"],
];
const PROBLEM: ImportCellError = {
  row: 5,
  column: "MRP",
  code: "money_has_separator",
  value: "12,5O",
};

let running = false;

function refuse(code: string, params: Record<string, string> = {}): never {
  const error: AppError = { kind: "validation", field: null, code, params };
  // eslint-disable-next-line @typescript-eslint/only-throw-error
  throw error;
}

function previewOf(request: ImportRequest): ImportPreview {
  const fields = FIELDS[request.entity];
  const mapping =
    request.mapping ??
    fields.map((each) => {
      const auto: Record<string, number> = {
        sku: 0,
        name: 1,
        barcodes: 2,
        sale_price: 3,
        cost_price: 4,
      };
      return auto[each.key] ?? null;
    });
  const missing = fields.filter((each, index) => each.required && mapping[index] == null);
  const errors: ImportCellError[] =
    missing.length > 0
      ? missing.map((each) => ({
          row: 1,
          column: each.header,
          code: "column_not_mapped",
          value: "",
        }))
      : [PROBLEM];
  return {
    sheets: request.file.endsWith(".xlsx") ? ["Products", "Notes"] : [],
    headers: HEADERS,
    fields,
    mapping,
    total_rows: ROWS.length,
    rows:
      missing.length > 0
        ? []
        : ROWS.map((cells, index) => ({
            row: index + 2,
            action: index === 3 ? "skip" : index === 2 ? "update" : "create",
            cells: mapping.map((column) => (column == null ? "" : (cells[column] ?? ""))),
          })),
    errors,
    counts:
      missing.length > 0
        ? { created: 0, updated: 0, skipped: 0 }
        : { created: 2, updated: 1, skipped: 1 },
  };
}

function pictureFor(): string {
  return testReceiptPicture("mm58");
}

export const ioHandlers = {
  import_preview: (args?: Args): ImportPreview => {
    requirePermission("data_import");
    return previewOf(args?.request as ImportRequest);
  },
  import_commit: (args?: Args): ImportDone => {
    requirePermission("data_import");
    const request = args?.request as ImportRequest;
    const preview = previewOf(request);
    if (preview.errors.length > 0 && !request.skip_errors) {
      refuse("import_has_errors", { count: String(preview.errors.length) });
    }
    return {
      counts: preview.counts,
      backup_file: "/fake/backups/pos-20261005-140000-pre-import.sqlite",
      errors: preview.errors,
    };
  },
  import_progress: (): ImportProgress => {
    requirePermission("data_import");
    return { done: running ? 2 : 0, total: ROWS.length, running };
  },
  import_cancel: (): null => {
    requirePermission("data_import");
    running = false;
    return null;
  },
  import_errors_save: (): null => {
    requirePermission("data_import");
    return null;
  },
  template_download: (): null => {
    requirePermission("data_import");
    return null;
  },
  export_entity: (args?: Args): number => {
    requirePermission("data_export");
    const request = args?.request as ExportRequest;
    if (request.entity === "price_list_prices" && request.price_list_id === null) {
      refuse("price_list_needed");
    }
    return request.entity === "products" ? fakeCatalog().products.length : 3;
  },
  io_price_lists: (): Choice[] => [{ id: "01FAKEPRICELIST00000000001", name: "Wholesale" }],
  labels_pick: (args?: Args): LabelPicked => {
    requirePermission("labels_print");
    const pick = args?.pick as LabelPick;
    const catalog = fakeCatalog();
    const chosen = catalog.products.filter((product) => {
      if (product.archived_at !== null || product.kind === "variant_parent") {
        return false;
      }
      switch (pick.kind) {
        case "products":
          return pick.ids.includes(product.id);
        case "category":
          return product.category_id === pick.id;
        case "price_changed_since":
          return product.name.startsWith("Milk");
        case "delivery":
          return false;
      }
    });
    const items: LabelItem[] = chosen.map((product) => ({
      product_id: product.id,
      name: product.name,
      sku: product.sku,
      price: product.sale_price,
      copies: 1,
    }));
    return { items, currency: currency() };
  },
  labels_preview: (args?: Args): LabelPicture[] => {
    requirePermission("labels_print");
    const request = args?.request as LabelRequest;
    if (request.settings.width_mm < 20 || request.settings.width_mm > 120) {
      refuse("label_size_out_of_range");
    }
    return request.items.map((item) => ({
      product_id: item.product_id,
      copies: item.copies,
      picture: pictureFor(),
    }));
  },
  labels_print: (args?: Args): LabelsPrinted => {
    requirePermission("labels_print");
    const request = args?.request as LabelRequest;
    const labels = request.items.reduce((sum, item) => sum + item.copies, 0);
    for (const item of request.items) {
      fakePreviewPrint({
        pictures: Array.from({ length: item.copies }, pictureFor),
        drawer_opened: false,
        saved_files: [],
      });
    }
    return { labels, to_preview: true };
  },
} satisfies Record<string, (args?: Args) => unknown>;
