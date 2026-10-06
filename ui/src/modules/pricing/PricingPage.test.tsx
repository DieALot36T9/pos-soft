import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { customerHandlers } from "../../shared/bridge/mock/customers";
import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { PricingPage } from "./pages/PricingPage";

// The pricing screens against the fake bridge (PRC-02 to PRC-04).

function renderPage() {
  render(
    <TooltipProvider>
      <PricingPage />
    </TooltipProvider>,
  );
}

describe("PricingPage (fake bridge)", () => {
  beforeEach(() => {
    resetFakeShop("set_up", true);
  });

  it("prc_02: adds a price list and sets a product's price in it", async () => {
    renderPage();
    expect(await screen.findByText("Wholesale")).toBeVisible();
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Staff");
    await userEvent.click(screen.getByRole("button", { name: /Save/ }));
    expect(await screen.findByText("Staff")).toBeVisible();

    const staffRow = screen.getByText("Staff").closest("tr") as HTMLElement;
    await userEvent.click(
      within(staffRow).getByRole("button", { name: "See and change the prices in this list." }),
    );
    const prices = await screen.findByRole("region", { name: "Prices in Staff" });
    expect(within(prices).getByText("No prices in this list yet.")).toBeVisible();
    await userEvent.type(within(prices).getByRole("textbox", { name: "Find a product" }), "milk");
    await userEvent.click(await within(prices).findByRole("button", { name: "Milk 1L" }));
    await userEvent.type(within(prices).getByRole("textbox", { name: "Price in this list" }), "80");
    await userEvent.click(within(prices).getByRole("button", { name: /Set price/ }));
    expect(await within(prices).findByText("৳80.00")).toBeVisible();
    expect(within(prices).getByText("৳90.00")).toBeVisible();
  });

  it("prc_02: a name already used is refused under the field", async () => {
    renderPage();
    await screen.findByText("Wholesale");
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Wholesale");
    await userEvent.click(screen.getByRole("button", { name: /Save/ }));
    expect(await screen.findByRole("alert")).toBeVisible();
  });

  it("prc_03: adds a tier with its discount and archives it", async () => {
    renderPage();
    await userEvent.click(await screen.findByRole("tab", { name: "Tiers" }));
    expect(await screen.findByText("Gold")).toBeVisible();
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Silver");
    const discount = screen.getByRole("textbox", { name: "Discount %" });
    await userEvent.clear(discount);
    await userEvent.type(discount, "2.5");
    await userEvent.click(screen.getByRole("button", { name: /Save/ }));
    const silver = (await screen.findByText("Silver")).closest("tr") as HTMLElement;
    expect(within(silver).getByText("2.5%")).toBeVisible();
    await userEvent.click(within(silver).getByRole("button", { name: /Archive this tier/ }));
    expect(await within(silver).findByText("Archived")).toBeVisible();
  });

  it("prc_04: adds a buy 2 get 1 promotion on a product and switches it off", async () => {
    renderPage();
    await userEvent.click(await screen.findByRole("tab", { name: "Promotions" }));
    await screen.findByText("No promotions yet. Add one below.");
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Milk deal");
    await userEvent.selectOptions(
      screen.getByRole("combobox", { name: "What it gives" }),
      "buy_x_get_y",
    );
    await userEvent.type(screen.getByRole("textbox", { name: "Buy" }), "2");
    await userEvent.type(screen.getByRole("textbox", { name: "Get free" }), "1");
    await userEvent.type(screen.getByRole("textbox", { name: "Find a product" }), "milk");
    await userEvent.click(await screen.findByRole("button", { name: "Milk 1L" }));
    await userEvent.click(screen.getByRole("button", { name: /Add product/ }));
    expect(screen.getByText("Product: Milk 1L")).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: /Save/ }));
    const row = (await screen.findByText("Buy 2, get 1 free")).closest("tr") as HTMLElement;
    expect(within(row).getByText("On")).toBeVisible();
    await userEvent.click(within(row).getByRole("button", { name: /Switch this promotion off/ }));
    expect(await within(row).findByText("Off")).toBeVisible();
    expect(customerHandlers.promotion_list()[0]?.targets).toHaveLength(1);
  });

  it("prc_04: a promotion with nothing to cover is refused", async () => {
    renderPage();
    await userEvent.click(await screen.findByRole("tab", { name: "Promotions" }));
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Empty");
    await userEvent.type(screen.getByRole("textbox", { name: "Percent off" }), "10");
    await userEvent.click(screen.getByRole("button", { name: /Save/ }));
    expect(
      await screen.findByText("Choose at least one product, category or brand."),
    ).toBeVisible();
  });
});
