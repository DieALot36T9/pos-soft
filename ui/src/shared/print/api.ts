import type { Printable } from "../../generated/Printable";
import type { PrintResult } from "../../generated/PrintResult";
import { call } from "../bridge";

// Sends a printout that failed to the receipt printer again (PAY-09).
export const retryPrint = (printable: Printable) => call<PrintResult>("print_retry", { printable });
// The failed printout as pictures, for the computer's print dialog (HW-07).
export const systemPrinterPictures = (printable: Printable) =>
  call<string[]>("print_system_pictures", { printable });
// Hides the banner: the printout no longer waits for "Retry".
export const dismissPrint = (printable: Printable) => call<null>("print_dismiss", { printable });
// Opens the computer's print dialog; only the picture area prints (ADR-013).
export const printWindow = () => call<null>("print_window");
