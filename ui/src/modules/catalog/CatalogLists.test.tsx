import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it } from "vitest";

import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { BrandsPage } from "./pages/BrandsPage";
import { CategoriesPage } from "./pages/CategoriesPage";
import { UnitsPage } from "./pages/UnitsPage";

function renderWithTips(page: ReactNode) {
  render(<TooltipProvider>{page}</TooltipProvider>);
}

describe("Units, Categories and Brands (fake bridge)", () => {
  beforeEach(() => resetFakeShop("set_up", true));

  it("cat_02: adds a unit; built-in units can't be archived", async () => {
    renderWithTips(<UnitsPage />);
    const piece = (await screen.findByText("Piece")).closest("tr") as HTMLElement;
    expect(within(piece).getByText("Comes with the app")).toBeVisible();
    expect(within(piece).queryByRole("button", { name: /Archive this unit/ })).toBeNull();

    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Dozen");
    await userEvent.type(screen.getByRole("textbox", { name: "Sign" }), "dz");
    await userEvent.click(screen.getByRole("button", { name: /Save unit/ }));
    expect(await screen.findByText("Dozen")).toBeVisible();
  });

  it("cat_03: a sub-category shows under its parent; a category with sub-categories can't be archived", async () => {
    renderWithTips(<CategoriesPage />);
    await within(await screen.findByRole("table")).findByText("Dairy");
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Milk");
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Inside" }), "Dairy");
    await userEvent.click(screen.getByRole("button", { name: /Save category/ }));
    const table = screen.getByRole("table");
    expect(await within(table).findByText("Dairy › Milk")).toBeVisible();

    const dairy = within(table).getByText("Dairy").closest("tr") as HTMLElement;
    await userEvent.click(within(dairy).getByRole("button", { name: /Archive this category/ }));
    expect(await screen.findByText(/Dairy has sub-categories/)).toBeVisible();
  });

  it("cat_03: brands are unique and can be archived", async () => {
    renderWithTips(<BrandsPage />);
    await screen.findByText("Farm Fresh");
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "farm fresh{Enter}");
    expect(await screen.findByText("Another brand is already called Farm Fresh.")).toBeVisible();

    const row = screen.getByText("Farm Fresh").closest("tr") as HTMLElement;
    await userEvent.click(within(row).getByRole("button", { name: /Archive this brand/ }));
    expect(await screen.findByText("No brands yet.")).toBeVisible();
  });
});
