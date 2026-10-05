import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { BackupsPage } from "./pages/BackupsPage";
import { RecoveryPage } from "./pages/RecoveryPage";

function renderWith(page: "backups" | "recovery") {
  render(
    <TooltipProvider>{page === "backups" ? <BackupsPage /> : <RecoveryPage />}</TooltipProvider>,
  );
}

describe("Backups (fake bridge)", () => {
  beforeEach(() => resetFakeShop("set_up", true));

  it("bak_04: Back up now adds a checked backup to the list", async () => {
    renderWith("backups");
    await screen.findByText("2026-10-04 20:00:00");
    await userEvent.click(screen.getByRole("button", { name: /Back up now/ }));
    expect(await screen.findByText(/Backed up to/)).toBeVisible();
    expect(screen.getAllByText("Checked")).toHaveLength(2);
    expect(screen.getByText("By hand")).toBeVisible();
  });

  it("bak_05: restore asks first and names the backup's date", async () => {
    renderWith("backups");
    await screen.findByText("2026-10-04 20:00:00");
    await userEvent.click(screen.getByRole("button", { name: /Put this backup back/ }));
    const dialog = await screen.findByRole("alertdialog");
    expect(dialog).toHaveTextContent("Restore the backup from 2026-10-04 20:00:00?");
    await userEvent.click(screen.getByRole("button", { name: /^Cancel/ }));
    expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument();
  });

  it("bak_06: the recovery screen offers the newest checked backup", async () => {
    renderWith("recovery");
    expect(
      await screen.findByRole("heading", { name: "The shop's data is damaged" }),
    ).toBeVisible();
    await userEvent.click(await screen.findByRole("button", { name: /Restore the newest backup/ }));
    expect(await screen.findAllByText(/The app will restart/)).not.toHaveLength(0);
  });
});
