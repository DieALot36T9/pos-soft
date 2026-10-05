import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { TaxRatesPage } from "./pages/TaxRatesPage";

function renderPage() {
  render(
    <TooltipProvider>
      <TaxRatesPage />
    </TooltipProvider>,
  );
}

describe("TaxRatesPage (fake bridge)", () => {
  beforeEach(() => resetFakeShop("set_up", true));

  it("tax_02: adds a rate with a decimal percentage", async () => {
    renderPage();
    await screen.findByText("Standard 15%");
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Reduced 7.5%");
    await userEvent.type(screen.getByRole("textbox", { name: "Percent" }), "7.5");
    await userEvent.click(screen.getByRole("button", { name: /Save rate/ }));

    const row = (await screen.findByText("Reduced 7.5%")).closest("tr");
    expect(row).not.toBeNull();
    expect(within(row as HTMLElement).getByText("7.5%")).toBeVisible();
  });

  it("tax_02: refuses a percentage with too many decimals before asking Rust", async () => {
    renderPage();
    await userEvent.type(await screen.findByRole("textbox", { name: "Name" }), "Odd");
    await userEvent.type(screen.getByRole("textbox", { name: "Percent" }), "1.234");
    await userEvent.click(screen.getByRole("button", { name: /Save rate/ }));
    expect(await screen.findByText(/at most 2 decimals/)).toBeVisible();
  });

  it("tax_10: deactivates a rate, which then shows only with inactive rates", async () => {
    renderPage();
    await screen.findByText("Standard 15%");
    await userEvent.click(screen.getByRole("button", { name: /Make this rate inactive/ }));
    expect(await screen.findByText(/No tax rates yet/)).toBeVisible();

    await userEvent.click(screen.getByRole("checkbox", { name: "Show inactive rates" }));
    expect(await screen.findByText("Inactive")).toBeVisible();
  });
});
