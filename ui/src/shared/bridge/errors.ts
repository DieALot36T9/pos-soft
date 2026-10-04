import type { TFunction } from "i18next";

import type { AppError } from "../../generated/AppError";
import { errors } from "../text/en/errors";

// The error codes that have a message in shared/text/en/errors.ts.
type ErrorCode = keyof typeof errors;

// True when a value looks like an AppError sent by Rust.
function isAppError(value: unknown): value is AppError {
  return typeof value === "object" && value !== null && "kind" in value;
}

// True when Rust sent a code we have a message for. Every language has the same keys as English.
function isKnownCode(code: string): code is ErrorCode {
  return Object.hasOwn(errors, code);
}

// Turns anything a command failed with into a plain-English message for the user.
// `t` is the "errors" namespace: const { t } = useTranslation("errors").
// Technical details are never shown; they go to the log.
export function errorMessage(error: unknown, t: TFunction<"errors">): string {
  if (!isAppError(error)) {
    return t("unknown");
  }
  switch (error.kind) {
    case "validation":
    case "conflict":
      return isKnownCode(error.code) ? t(error.code, { replace: error.params }) : t("unknown");
    case "not_found":
      return t("not_found");
    case "permission_denied":
      return t("permission_denied");
    case "override_required":
      return t("override_required");
    case "hardware":
      return t("hardware");
    case "internal":
      return t("internal", { ref: error.id.slice(-6) });
  }
}
