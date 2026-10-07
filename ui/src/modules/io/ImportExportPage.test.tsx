import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import type { Permission } from "../../generated/Permission";
import { allPermissions } from "../../shared/bridge/mock/people";
import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { ImportExportPage } from "./pages/ImportExportPage";

// The Import / Export screen against the fake bridge (Req - Import and Export). The fake
// file has four product rows and one problem; Rust's tests read real files.

function renderPage(permissions: Permission[] = allPermissions) {
  render(
    <TooltipProvider>
      <ImportExportPage permissions={permissions} />
    </TooltipProvider>,
  );
}

const button = (name: RegExp) => screen.getByRole("button", { name });

describe("ImportExportPage (fake bridge)", () => {
  beforeEach(() => {
    resetFakeShop("set_up", true);
  });

  it("io_01: a chosen file is matched by its headers and every problem is listed", async () => {
    renderPage();
    await userEvent.click(button(/^Choose file/));
    expect(await screen.findByText("File: /fake/imports/products.csv")).toBeVisible();
    expect(screen.getByRole("combobox", { name: "Sale price (needed)" })).toHaveValue("3");
    expect(screen.getByText("2 new, 1 changed, 1 left out, of 4 rows.")).toBeVisible();
    // Once in the preview rows, once in the problem list.
    expect(screen.getAllByRole("cell", { name: "12,5O" })).toHaveLength(2);
    expect(
      screen.getByRole("cell", {
        name: "Leave out thousands signs. Example: 1250.00, not 1,250.00",
      }),
    ).toBeVisible();
  });

  it("io_01: import waits for the problems to be fixed or skipped, then shows the summary", async () => {
    renderPage();
    await userEvent.click(button(/^Choose file/));
    await screen.findByText("File: /fake/imports/products.csv");
    expect(button(/^Import$/)).toHaveAttribute("aria-disabled", "true");
    await userEvent.click(screen.getByRole("checkbox", { name: "Skip rows with errors" }));
    await userEvent.click(button(/^Import$/));
    expect(await screen.findByText("2 new, 1 changed, 1 left out.")).toBeVisible();
    expect(screen.getByText(/pre-import\.sqlite/)).toBeVisible();
    await userEvent.click(button(/^Save error file/));
    expect(await screen.findByText("The error file was saved.")).toBeVisible();
  });

  it("io_01: a needed field without a column is shown as a problem", async () => {
    renderPage();
    await userEvent.click(button(/^Choose file/));
    const price = await screen.findByRole("combobox", { name: "Sale price (needed)" });
    await userEvent.selectOptions(price, "");
    await userEvent.click(button(/^Check file/));
    expect(
      await screen.findByRole("cell", {
        name: "No column is chosen for this field. Choose one above.",
      }),
    ).toBeVisible();
  });

  it("io_03: exports save a file and say how many rows", async () => {
    renderPage();
    await userEvent.click(screen.getByRole("tab", { name: "Export" }));
    await userEvent.selectOptions(
      screen.getByRole("combobox", { name: "What to export" }),
      "customers",
    );
    await userEvent.click(button(/^Export$/));
    expect(await screen.findByText("3 rows were saved.")).toBeVisible();
  });

  it("an export-only role sees only the Export tab", () => {
    renderPage(["data_export"]);
    expect(screen.queryByRole("tab", { name: "Import" })).toBeNull();
    expect(screen.getByRole("combobox", { name: "What to export" })).toBeVisible();
  });
});
