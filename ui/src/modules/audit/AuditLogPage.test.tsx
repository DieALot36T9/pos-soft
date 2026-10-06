import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { peopleHandlers, signInFakeAs } from "../../shared/bridge/mock/people";
import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { AuditLogPage } from "./pages/AuditLogPage";

function renderPage() {
  render(
    <TooltipProvider>
      <AuditLogPage />
    </TooltipProvider>,
  );
}

describe("Audit log (fake bridge)", () => {
  beforeEach(() => {
    resetFakeShop("set_up", false);
    // The cashier logs in and out, then the owner logs in.
    const tiles = peopleHandlers.login_tiles();
    const rina = tiles.find((tile) => tile.username === "rina");
    const owner = tiles.find((tile) => tile.username === "owner");
    peopleHandlers.login({ userId: rina?.id, secret: "1234" });
    peopleHandlers.logout();
    peopleHandlers.login({ userId: owner?.id, secret: "shop-owner-1" });
  });

  it("usr_08: shows each action with the user's name, newest first", async () => {
    renderPage();
    const rows = await screen.findAllByRole("row");
    // The heading row, then: owner logged in, rina logged out, rina logged in.
    expect(rows).toHaveLength(4);
    expect(rows[1]).toHaveTextContent("Shop Owner");
    expect(rows[1]).toHaveTextContent("Logged in");
    expect(rows[2]).toHaveTextContent("Logged out");
  });

  it("usr_08: filters by action", async () => {
    renderPage();
    await screen.findAllByRole("row");
    await userEvent.selectOptions(screen.getByLabelText("Action"), "session.logout");
    await userEvent.click(screen.getByRole("button", { name: /^Show/ }));
    await screen.findByText("Actions 1 to 1");
    expect(screen.getAllByRole("row")).toHaveLength(2);
  });

  it("usr_08: a wrong date is caught before asking Rust", async () => {
    renderPage();
    await userEvent.type(screen.getByLabelText("From"), "5 October");
    await userEvent.click(screen.getByRole("button", { name: /^Show/ }));
    expect(await screen.findByText(/Type the date as year-month-day/)).toBeVisible();
  });

  it("usr_08: exports the matching actions to Excel", async () => {
    renderPage();
    await screen.findAllByRole("row");
    await userEvent.click(screen.getByRole("button", { name: /Export to Excel/ }));
    expect(await screen.findByText("Saved 3 actions to the Excel file.")).toBeVisible();
  });

  it("usr_08: a cashier can't see the log", async () => {
    signInFakeAs("rina");
    renderPage();
    expect(await screen.findByText("Your role can't do this. Ask an admin.")).toBeVisible();
  });
});
