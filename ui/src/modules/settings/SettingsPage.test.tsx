import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { SettingsPage } from "./pages/SettingsPage";

function renderPage() {
  render(
    <TooltipProvider>
      <SettingsPage />
    </TooltipProvider>,
  );
}

describe("SettingsPage (fake bridge)", () => {
  beforeEach(() => resetFakeShop("set_up", true));

  it("set_13: changes the shop name and says it is saved", async () => {
    renderPage();
    const name = await screen.findByRole("textbox", { name: "Shop name" });
    expect(name).toHaveValue("Rahim Store");
    await userEvent.clear(name);
    await userEvent.type(name, "Rahim Super Store");
    await userEvent.click(screen.getByRole("button", { name: /^Save/ }));
    expect(await screen.findByText("Saved.")).toBeVisible();
  });

  it("set_05: the money section shows the live preview", async () => {
    renderPage();
    await userEvent.click(await screen.findByRole("button", { name: "Money and rounding" }));
    expect(await screen.findByText("৳12,34,567.50")).toBeVisible();
    await userEvent.selectOptions(
      screen.getByRole("combobox", { name: "Digit grouping" }),
      "western",
    );
    expect(await screen.findByText("৳1,234,567.50")).toBeVisible();
  });

  it("tax_04: changing the tax mode warns that only future sales change", async () => {
    renderPage();
    await userEvent.click(await screen.findByRole("button", { name: "Tax" }));
    await userEvent.selectOptions(
      await screen.findByRole("combobox", { name: "Shelf prices" }),
      "exclusive",
    );
    expect(screen.getByText(/affects only future sales/)).toBeVisible();
  });

  it("bak_07: the detailed-logging switch saves at once", async () => {
    renderPage();
    await userEvent.click(await screen.findByRole("button", { name: "Logs" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Detailed logging" }));
    expect(await screen.findByText(/works right away/)).toBeVisible();
  });

  it("set_13: without a signed-in admin, settings are refused", async () => {
    resetFakeShop("set_up", false);
    renderPage();
    expect(await screen.findByText("Your role can't do this. Ask an admin.")).toBeVisible();
  });
});
