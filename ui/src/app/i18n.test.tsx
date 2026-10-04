import { render, screen } from "@testing-library/react";
import { useTranslation } from "react-i18next";
import { describe, expect, it } from "vitest";

import { i18n } from "./i18n";

function CheckLabel() {
  const { t } = useTranslation("app");
  return <p>{t("checkButton.label")}</p>;
}

describe("screen text (i18next)", () => {
  it("gives a component its module's English text", () => {
    render(<CheckLabel />);
    expect(screen.getByText("Check app")).toBeInTheDocument();
  });

  it("knows every namespace: shared text and each module's text/en.ts", () => {
    expect(i18n.getFixedT("en", "common")("appName")).toBe("Supershop POS");
    expect(i18n.getFixedT("en", "errors")("unknown")).toBe("Something went wrong. Try again.");
    expect(i18n.getFixedT("en", "app")("title")).toBe("Welcome");
  });

  // These lines prove that typed keys work: if TypeScript ever stopped refusing them,
  // the unused @ts-expect-error would itself fail `pnpm typecheck`.
  it("refuses keys and namespaces that don't exist (checked by pnpm typecheck)", () => {
    const t = i18n.getFixedT("en", "app");
    // @ts-expect-error "noSuchKey" is not in app/text/en.ts.
    expect(t("noSuchKey")).toBe("noSuchKey");
    // @ts-expect-error "checkButton.colour" is not in app/text/en.ts.
    expect(t("checkButton.colour")).toBe("checkButton.colour");
    // @ts-expect-error there is no "no_such_module" namespace.
    expect(i18n.getFixedT("en", "no_such_module")).toBeTypeOf("function");
  });
});
