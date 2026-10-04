import { describe, expect, it } from "vitest";

import { errors } from "../text/en/errors";
import { errorMessage } from "./errors";

describe("errorMessage", () => {
  it("looks up the message for a validation code and fills in details", () => {
    const error = {
      kind: "validation",
      field: null,
      code: "money_too_many_decimals",
      params: { max_decimals: "2" },
    };
    expect(errorMessage(error, errors)).toBe("Use at most 2 digits after the point.");
  });

  it("shows only a short reference for a bug", () => {
    const error = { kind: "internal", id: "01HX5Z3J8Q9W2E4R6T8Y0V1K2M" };
    expect(errorMessage(error, errors)).toContain("ref 0V1K2M");
  });

  it("has a safe message for anything unexpected", () => {
    expect(errorMessage(new Error("boom"), errors)).toBe(errors.unknown);
    expect(
      errorMessage({ kind: "validation", field: null, code: "nope", params: {} }, errors),
    ).toBe(errors.unknown);
  });
});
