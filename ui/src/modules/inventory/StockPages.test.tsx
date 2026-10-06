import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { signInFakeAs } from "../../shared/bridge/mock/people";
import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { StockAlertsCard } from "./components/StockAlertsCard";
import { StockCountsPage } from "./pages/StockCountsPage";
import { StockLevelsPage } from "./pages/StockLevelsPage";

const textbox = (name: string | RegExp) => screen.getByRole("textbox", { name });
const select = (name: string | RegExp) => screen.getByRole("combobox", { name });
const button = (name: RegExp) => screen.getByRole("button", { name });

function renderLevels(canAdjust = true) {
  render(
    <TooltipProvider>
      <StockLevelsPage canAdjust={canAdjust} />
    </TooltipProvider>,
  );
}

function renderCounts(canManage = true) {
  render(
    <TooltipProvider>
      <StockCountsPage canManage={canManage} />
    </TooltipProvider>,
  );
}

async function openStock(name: string) {
  renderLevels();
  const row = (await screen.findByText(name)).closest("tr") as HTMLElement;
  await userEvent.click(within(row).getByRole("button", { name: /Open the stock/ }));
  await screen.findByRole("heading", { name });
}

describe("StockLevelsPage (fake bridge)", () => {
  beforeEach(() => resetFakeShop("set_up", true));

  it("inv_01: lists stock with its value, low and expiring marks", async () => {
    renderLevels();
    const milk = (await screen.findByText("Milk 1L")).closest("tr") as HTMLElement;
    expect(within(milk).getByText("24 pc")).toBeVisible();
    expect(within(milk).getByText("৳1,800.00")).toBeVisible();
    expect(within(milk).getByText("Low")).toBeVisible();
    expect(within(milk).getByText("Expiring")).toBeVisible();
    const bread = screen.getByText("Bread").closest("tr") as HTMLElement;
    expect(within(bread).getByText("OK")).toBeVisible();
    expect(screen.getByText(/3 of 3 products · stock value/)).toBeVisible();
  });

  it("inv_03: shows only low stock when asked", async () => {
    renderLevels();
    await screen.findByText("Milk 1L");
    await userEvent.selectOptions(select("Show"), "Low stock");
    expect(await screen.findByText(/1 of 1 products/)).toBeVisible();
    expect(screen.queryByText("Bread")).toBeNull();
  });

  it("inv_10: verify stock data says every level matches", async () => {
    renderLevels();
    await userEvent.click(await screen.findByRole("button", { name: /Verify stock data/ }));
    expect(await screen.findByText(/every level matches its history/)).toBeVisible();
  });

  it("hides verify for a user who can't adjust stock", async () => {
    renderLevels(false);
    await screen.findByText("Milk 1L");
    expect(screen.queryByRole("button", { name: /Verify stock data/ })).toBeNull();
  });
});

describe("StockProductPage (fake bridge)", () => {
  beforeEach(() => resetFakeShop("set_up", true));

  it("inv_05: shows the batches, first expiry first, with their marks", async () => {
    await openStock("Milk 1L");
    const soon = screen.getAllByText("M-0921")[0]?.closest("tr") as HTMLElement;
    expect(within(soon).getByText("Soon")).toBeVisible();
    const later = screen.getAllByText("M-1001")[0]?.closest("tr") as HTMLElement;
    expect(within(later).getByText("Good")).toBeVisible();
  });

  it("inv_02: takes stock out with a reason and shows it in the history", async () => {
    await openStock("Bread");
    await userEvent.click(button(/Adjust stock/));
    await userEvent.selectOptions(select("Way"), "Stock out (−)");
    await userEvent.type(textbox("Quantity"), "2");
    await userEvent.selectOptions(select("Reason"), "Damage");
    await userEvent.click(button(/^Save/));

    expect(await screen.findByText("Stock saved. On hand now: 10 pc.")).toBeVisible();
    expect(screen.getByTestId("on-hand")).toHaveTextContent("On hand: 10 pc");
    const row = (await screen.findByText("-2")).closest("tr") as HTMLElement;
    expect(within(row).getByText("Adjustment")).toBeVisible();
    expect(within(row).getByText("Damage")).toBeVisible();
  });

  it("inv_02: the reason Other needs a note", async () => {
    await openStock("Bread");
    await userEvent.click(button(/Adjust stock/));
    await userEvent.type(textbox("Quantity"), "1");
    await userEvent.selectOptions(select("Reason"), "Other");
    await userEvent.click(button(/^Save/));
    expect(await screen.findByText(/Type a note that says what happened/)).toBeVisible();
  });

  it("inv_02: the reasons fit the way the stock goes", async () => {
    await openStock("Bread");
    await userEvent.click(button(/Adjust stock/));
    const reasons = () =>
      within(select("Reason"))
        .getAllByRole("option")
        .map((option) => option.textContent);
    await vi.waitFor(() => expect(reasons()).toContain("Opening stock"));
    expect(reasons()).not.toContain("Damage");
    await userEvent.selectOptions(select("Way"), "Stock out (−)");
    expect(reasons()).toContain("Damage");
    expect(reasons()).not.toContain("Opening stock");
  });
});

describe("StockCountsPage (fake bridge)", () => {
  beforeEach(() => resetFakeShop("set_up", true));

  it("inv_06: starts a full count, counts by scanning, reviews and posts", async () => {
    renderCounts();
    await userEvent.type(await screen.findByRole("textbox", { name: "Name" }), "October count");
    await userEvent.click(button(/Start count/));
    expect(await screen.findByRole("heading", { name: /SC-000001: October count/ })).toBeVisible();

    // Soap: scanned twice, 30 then 8 → 38 counted.
    await userEvent.type(textbox(/Scan or type/), "8902222222227");
    await userEvent.clear(textbox("Quantity"));
    await userEvent.type(textbox("Quantity"), "30{Enter}");
    expect(await screen.findByText("Counted 30 pc of Soap.")).toBeVisible();
    await userEvent.type(textbox(/Scan or type/), "8902222222227");
    await userEvent.clear(textbox("Quantity"));
    await userEvent.type(textbox("Quantity"), "8{Enter}");
    const soap = (await screen.findByText("38 pc")).closest("tr") as HTMLElement;
    expect(within(soap).getByText("Soap")).toBeVisible();

    await userEvent.click(button(/Review differences/));
    const row = (await screen.findByText("-2")).closest("tr") as HTMLElement;
    expect(within(row).getByText("Soap")).toBeVisible();
    expect(screen.getByText(/2 products with stock were not counted/)).toBeVisible();

    // Post waits for the confirmation about uncounted products.
    expect(button(/Post count/)).toHaveAttribute("aria-disabled", "true");
    await userEvent.click(screen.getByRole("checkbox", { name: /Set uncounted products to 0/ }));
    await userEvent.click(button(/Post count/));
    expect(await screen.findByText(/Posted: 3 stock changes/)).toBeVisible();
    expect(screen.queryByRole("button", { name: /Post count/ })).toBeNull();
  });

  it("inv_06: a code that matches nothing says so", async () => {
    renderCounts();
    await userEvent.type(await screen.findByRole("textbox", { name: "Name" }), "Shelf");
    await userEvent.click(button(/Start count/));
    await userEvent.type(
      await screen.findByRole("textbox", { name: /Scan or type/ }),
      "NOPE{Enter}",
    );
    expect(await screen.findByText(/not found|wasn't found|no longer/i)).toBeVisible();
  });

  it("a cashier counts but can't start or review counts", async () => {
    renderCounts();
    await userEvent.type(await screen.findByRole("textbox", { name: "Name" }), "Aisle 3");
    await userEvent.selectOptions(select("Covers"), "Part of the shop");
    await userEvent.click(button(/Start count/));
    await screen.findByRole("heading", { name: /SC-000001/ });
    signInFakeAs("rina");
    document.body.innerHTML = "";
    renderCounts(false);
    expect(screen.queryByRole("button", { name: /Start count/ })).toBeNull();
    const row = (await screen.findByText("Aisle 3")).closest("tr") as HTMLElement;
    await userEvent.click(within(row).getByRole("button", { name: /Open count/ }));
    await screen.findByRole("heading", { name: /SC-000001/ });
    expect(screen.queryByRole("button", { name: /Review differences/ })).toBeNull();
    expect(textbox(/Scan or type/)).toBeVisible();
  });
});

describe("StockAlertsCard (fake bridge)", () => {
  beforeEach(() => resetFakeShop("set_up", true));

  it("inv_03, inv_05: shows low stock and batches near expiry", async () => {
    const open = vi.fn();
    render(
      <TooltipProvider>
        <StockAlertsCard onOpenStock={open} />
      </TooltipProvider>,
    );
    expect(await screen.findByText("1 products are below their reorder level")).toBeVisible();
    expect(screen.getByText("Milk 1L: 24 pc (reorder below 30)")).toBeVisible();
    expect(screen.getByText("Milk 1L: 10 pc, expires 2026-10-12")).toBeVisible();
    await userEvent.click(button(/Open stock/));
    expect(open).toHaveBeenCalled();
  });
});
