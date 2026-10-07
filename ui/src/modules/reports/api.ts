import type { Choice } from "../../generated/Choice";
import type { PrintResult } from "../../generated/PrintResult";
import type { ReportChoices } from "../../generated/ReportChoices";
import type { ReportRequest } from "../../generated/ReportRequest";
import type { ReportTable } from "../../generated/ReportTable";
import type { ShiftList } from "../../generated/ShiftList";
import { call } from "../../shared/bridge";

// Tills, cashiers, categories, brands, suppliers, today's business day and the money format.
export const loadChoices = () => call<ReportChoices>("report_choices");
// Customers whose name or phone matches, for the customer statement (at most 20).
export const findCustomers = (text: string) => call<Choice[]>("report_customer_search", { text });
// Shifts, newest first, with the money format. A cashier gets only their own (RPT-10).
export const loadShifts = () => call<ShiftList>("report_shifts");
// Runs one report. Every number in it comes from Rust.
export const runReport = (request: ReportRequest) => call<ReportTable>("report_run", { request });
// Saves the report as an Excel file (RPT-09).
export const exportReport = (request: ReportRequest, file: string) =>
  call<null>("report_export_xlsx", { request, file });
// Prints the report on the receipt printer (RPT-01).
export const printReport = (request: ReportRequest) =>
  call<PrintResult>("report_print", { request });
