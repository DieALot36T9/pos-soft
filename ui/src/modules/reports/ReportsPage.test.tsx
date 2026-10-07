import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { allPermissions, signInFakeAs } from "../../shared/bridge/mock/people";
import { resetFakeShop } from "../../shared/bridge/mock/shop";
import type { Permission } from "../../generated/Permission";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { ReportsPage } from "./pages/ReportsPage";

// The Reports screen against the fake bridge (Req - Reports). The fake reports are fixed
// demo tables; Rust's tests check the real numbers.

function renderPage(permissions: Permission[] = allPermissions) {
  render(
    <TooltipProvider>
      <ReportsPage permissions={permissions} />
    </TooltipProvider>,
  );
}

const button = (name: RegExp) => screen.getByRole("button", { name });

describe("ReportsPage (fake bridge)", () => {
  beforeEach(() => {
    resetFakeShop("set_up", true);
  });

  it("rpt_01: shows the daily summary for today in one generic table", async () => {
    renderPage();
    expect(await screen.findByDisplayValue("2026-03-02")).toHaveAccessibleName("Business day");
    await userEvent.click(button(/^Show/));
    const report = await screen.findByRole("article", { name: "Daily summary" });
    expect(report).toHaveTextContent("Rahim Store");
    expect(report).toHaveTextContent("Day: 2026-03-02");
    expect(screen.getByRole("cell", { name: "Net sales" })).toBeVisible();
    expect(screen.getByRole("heading", { name: "Payments by method" })).toBeVisible();
  });

  it("rpt_03: the week preset fills the range, and a typed top must be a whole number", async () => {
    renderPage();
    await screen.findByDisplayValue("2026-03-02");
    await userEvent.click(button(/^Sales analysis/));
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Days" }), "this_month");
    expect(screen.getByRole("textbox", { name: "From" })).toHaveValue("2026-03-01");
    await userEvent.type(screen.getByRole("textbox", { name: "Top" }), "1.5");
    await userEvent.click(button(/^Show/));
    expect(await screen.findByText(/whole number for Top/)).toBeVisible();
  });

  it("rpt_09: print and Excel wait until the report is shown", async () => {
    renderPage();
    await screen.findByDisplayValue("2026-03-02");
    expect(button(/^Excel/)).toHaveAttribute("aria-disabled", "true");
    await userEvent.click(button(/^Show/));
    await screen.findByRole("article", { name: "Daily summary" });
    expect(button(/^Excel/)).toHaveAttribute("aria-disabled", "false");
    await userEvent.click(button(/^Print receipt/));
    expect(await screen.findByText("The report was printed.")).toBeVisible();
  });

  it("inv_04: negative stock shows in red", async () => {
    renderPage();
    await screen.findByDisplayValue("2026-03-02");
    await userEvent.click(button(/^Stock value/));
    await userEvent.click(button(/^Show/));
    expect(await screen.findByRole("cell", { name: "-2" })).toHaveClass("text-red");
  });

  it("rpt_10: a cashier sees only the shift reports of their own shifts", async () => {
    signInFakeAs("rina");
    renderPage(["shift_open_close_own"]);
    expect(await screen.findByRole("combobox", { name: "Shift" })).toBeVisible();
    expect(screen.queryByRole("button", { name: /^Daily summary/ })).toBeNull();
    expect(screen.getAllByRole("option")).toHaveLength(1);
    await userEvent.click(button(/^Show/));
    expect(await screen.findByRole("article", { name: "Z report" })).toBeVisible();
  });
});
