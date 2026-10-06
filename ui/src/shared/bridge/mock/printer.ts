import type { AppError } from "../../../generated/AppError";
import type { HardwareSettings } from "../../../generated/HardwareSettings";
import type { PreviewPrintout } from "../../../generated/PreviewPrintout";
import type { Printable } from "../../../generated/Printable";
import type { PrintResult } from "../../../generated/PrintResult";
// The approved printouts, made by Rust (crates/app/tests/golden).
import receiptCopy from "../../../../../crates/app/tests/golden/sale_receipt_copy_80mm.png";
import returnReceipt from "../../../../../crates/app/tests/golden/return_receipt_80mm.png";
import receipt from "../../../../../crates/app/tests/golden/sale_receipt_80mm.png";
import voidReceipt from "../../../../../crates/app/tests/golden/void_receipt_80mm.png";
import xReport from "../../../../../crates/app/tests/golden/x_report_80mm.png";
import zReport from "../../../../../crates/app/tests/golden/z_report_80mm.png";
import { currentSession } from "./people";

// A pretend receipt printer for the fake bridge. The browser has no printer, so only
// the Preview printer works: any other printer in the settings counts as offline,
// which is how the "Receipt not printed" banner can be tried (PAY-09).

export const pictures = { receipt, receiptCopy, xReport, zReport, returnReceipt, voidReceipt };

// What the fake Preview printer has printed and not yet handed out.
let waitingPrintouts: PreviewPrintout[] = [];
// Printouts that failed and wait for Retry: each with the picture it would print.
let unprinted: { printable: Printable; picture: string; drawer: boolean }[] = [];
let hardware: () => HardwareSettings = () => {
  throw new Error("The fake printer isn't connected to the fake shop.");
};

// shop.ts calls this once, so the printer follows the shop's hardware settings.
export function connectPrinter(shopHardware: () => HardwareSettings) {
  hardware = shopHardware;
}

// Starts the pretend printer again with nothing printed.
export function resetPrinter() {
  waitingPrintouts = [];
  unprinted = [];
}

function offline(): AppError | null {
  const printer = hardware().receipt_printer;
  return printer === null || printer.kind === "preview"
    ? null
    : {
        kind: "hardware",
        device: "printer",
        code: "not_connected",
        detail: "the fake bridge has only the Preview printer",
      };
}

function same(a: Printable, b: Printable) {
  return JSON.stringify(a) === JSON.stringify(b);
}

// Prints a picture, or remembers it for Retry when the printer is offline.
export function fakePrint(printable: Printable, picture: string, drawer: boolean): PrintResult {
  const problem = offline();
  unprinted = unprinted.filter((each) => !same(each.printable, printable));
  if (problem) {
    unprinted.push({ printable, picture, drawer });
    return { printable, printed: false, problem };
  }
  waitingPrintouts.push({ pictures: [picture], drawer_opened: drawer, saved_files: [] });
  return { printable, printed: true, problem: null };
}

// Opens the cash drawer through the printer (HW-04).
export function fakeDrawer(): PrintResult {
  const problem = offline();
  if (!problem) {
    waitingPrintouts.push({ pictures: [], drawer_opened: true, saved_files: [] });
  }
  return { printable: null, printed: problem === null, problem };
}

// Any picture into the Preview printer, as "Test print" does.
export function fakePreviewPrint(printout: PreviewPrintout) {
  waitingPrintouts.push(printout);
}

function waiting(printable: Printable) {
  if (!currentSession()) {
    const error: AppError = { kind: "conflict", code: "not_signed_in", params: {} };
    // eslint-disable-next-line @typescript-eslint/only-throw-error
    throw error;
  }
  const found = unprinted.find((each) => same(each.printable, printable));
  if (!found) {
    const error: AppError = { kind: "conflict", code: "print_not_waiting", params: {} };
    // eslint-disable-next-line @typescript-eslint/only-throw-error
    throw error;
  }
  return found;
}

type Args = Record<string, unknown>;

export const printerHandlers = {
  preview_take_printouts: (): PreviewPrintout[] => {
    const printouts = waitingPrintouts;
    waitingPrintouts = [];
    return printouts;
  },
  print_retry: (args?: Args): PrintResult => {
    const found = waiting(args?.printable as Printable);
    return fakePrint(found.printable, found.picture, found.drawer);
  },
  print_system_pictures: (args?: Args): string[] => [waiting(args?.printable as Printable).picture],
  print_dismiss: (args?: Args): null => {
    const found = waiting(args?.printable as Printable);
    unprinted = unprinted.filter((each) => each !== found);
    return null;
  },
} satisfies Record<string, (args?: Args) => unknown>;
