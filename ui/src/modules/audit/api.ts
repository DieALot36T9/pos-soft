import type { AuditFilter } from "../../generated/AuditFilter";
import type { AuditList } from "../../generated/AuditList";
import type { AuditPage } from "../../generated/AuditPage";
import { call } from "../../shared/bridge";

// One page of the audit log, newest first, with user names and every action name (USR-08).
export const queryAudit = (filter: AuditFilter, page: AuditPage) =>
  call<AuditList>("audit_query", { filter, page });
// Saves every event the filter finds as an Excel file. Answers how many were written.
export const exportAudit = (filter: AuditFilter, file: string) =>
  call<number>("audit_export", { filter, file });
