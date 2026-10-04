import type { AppError } from "../../generated/AppError";
import type { ErrorText } from "../text/en/errors";

// True when a value looks like an AppError sent by Rust.
function isAppError(value: unknown): value is AppError {
  return typeof value === "object" && value !== null && "kind" in value;
}

// Fills {name} gaps in a message with the error's details.
function fill(message: string, params: Record<string, string | undefined>): string {
  return message.replace(/\{(\w+)\}/g, (gap, name: string) => params[name] ?? gap);
}

// Turns anything a command failed with into a plain-English message for the user.
// Technical details are never shown; they go to the log.
export function errorMessage(error: unknown, text: ErrorText): string {
  if (!isAppError(error)) {
    return text.unknown;
  }
  switch (error.kind) {
    case "validation":
    case "conflict": {
      const message = (text as Record<string, string | undefined>)[error.code];
      return message ? fill(message, error.params) : text.unknown;
    }
    case "not_found":
      return text.not_found;
    case "permission_denied":
      return text.permission_denied;
    case "override_required":
      return text.override_required;
    case "hardware":
      return text.hardware;
    case "internal":
      return fill(text.internal, { ref: error.id.slice(-6) });
  }
}
