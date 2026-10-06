import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { resetFakeShop, shopHandlers } from "../../shared/bridge/mock/shop";
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

  it("set_17: switches on loyalty points and selling on credit", async () => {
    renderPage();
    await userEvent.click(await screen.findByRole("button", { name: "Loyalty and credit" }));
    await userEvent.click(
      await screen.findByRole("checkbox", { name: "Customers collect points" }),
    );
    await userEvent.click(screen.getByRole("checkbox", { name: "Customers may buy on credit" }));
    const limit = screen.getByRole("textbox", { name: "Credit limit for new customers" });
    await userEvent.clear(limit);
    await userEvent.type(limit, "500");
    await userEvent.click(screen.getByRole("button", { name: /^Save/ }));
    expect(await screen.findByText("Saved.")).toBeVisible();
    const saved = shopHandlers.settings_get();
    expect(saved.loyalty.enabled).toBe(true);
    expect(saved.credit).toEqual({ enabled: true, default_limit_minor: 50000 });
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

  it("set_10 hw_05: the printer and scale section saves a network printer and a scale", async () => {
    renderPage();
    await userEvent.click(await screen.findByRole("button", { name: "Printer and scale" }));
    await userEvent.selectOptions(
      await screen.findByRole("combobox", { name: "Receipt printer" }),
      "tcp",
    );
    await userEvent.type(screen.getByRole("textbox", { name: "IP address" }), "192.168.1.50");
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Scale" }), "cas");
    await userEvent.click(screen.getByRole("button", { name: /^Save/ }));
    expect(
      await screen.findAllByText("Type the scale's port and speed. Example: COM3 at 9600."),
    ).not.toHaveLength(0);
    await userEvent.type(screen.getByRole("textbox", { name: "Scale port" }), "COM4");
    await userEvent.click(screen.getByRole("button", { name: /^Save/ }));
    expect(await screen.findByText("Saved.")).toBeVisible();
    const hardware = shopHandlers.settings_get().hardware;
    expect(hardware.receipt_printer).toEqual({ kind: "tcp", host: "192.168.1.50", port: 9100 });
    expect(hardware.scale).toEqual({ port: "COM4", baud: 9600, profile: "cas" });
  });
});
