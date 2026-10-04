import i18next from "i18next";
import { describe, expect, it } from "vitest";

import { errors } from "../text/en/errors";
import { i18nOptions } from "../text/i18n";
import { errorMessage } from "./errors";

// Only the shared "errors" namespace, so this test doesn't need the app's set-up.
const text = i18next.createInstance();
void text.init({ ...i18nOptions, resources: { en: { errors } } });
const t = text.getFixedT("en", "errors");

describe("errorMessage", () => {
  it("looks up the message for a validation code and fills in details", () => {
    const error = {
      kind: "validation",
      field: null,
      code: "money_too_many_decimals",
      params: { max_decimals: "2" },
    };
    expect(errorMessage(error, t)).toBe("Use at most 2 digits after the point.");
  });

  it("shows only a short reference for a bug", () => {
    const error = { kind: "internal", id: "01HX5Z3J8Q9W2E4R6T8Y0V1K2M" };
    expect(errorMessage(error, t)).toContain("ref 0V1K2M");
  });

  it("has a safe message for anything unexpected", () => {
    expect(errorMessage(new Error("boom"), t)).toBe(errors.unknown);
    expect(errorMessage({ kind: "validation", field: null, code: "nope", params: {} }, t)).toBe(
      errors.unknown,
    );
  });
});
