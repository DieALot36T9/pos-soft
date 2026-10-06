import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { signInFakeAs } from "../shared/bridge/mock/people";
import { resetFakeShop } from "../shared/bridge/mock/shop";
import { TooltipProvider } from "../shared/ui/tooltip";
import { AppFrame } from "./AppFrame";

function renderFrame() {
  render(
    <TooltipProvider>
      <AppFrame />
    </TooltipProvider>,
  );
}

describe("AppFrame", () => {
  beforeEach(() => resetFakeShop("set_up", true));

  it("shows the app name and asks Rust (the fake bridge) for the version", async () => {
    renderFrame();
    expect(screen.getByRole("heading", { name: "Supershop POS" })).toBeInTheDocument();

    await userEvent.click(await screen.findByRole("button", { name: /Check app/ }));

    expect(await screen.findByText("0.1.0 (fake bridge)")).toBeInTheDocument();
  });

  it("set_01: a fresh install opens the setup wizard", async () => {
    resetFakeShop("fresh");
    renderFrame();
    expect(
      await screen.findByRole("heading", { name: "Welcome to Supershop POS" }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Main menu" })).not.toBeInTheDocument();
  });

  it("usr_01: after setup, a user picks their tile and types the PIN", async () => {
    resetFakeShop("set_up", false);
    renderFrame();
    await userEvent.click(await screen.findByRole("button", { name: /Rina Akter/ }));
    await userEvent.type(screen.getByLabelText("PIN"), "9999{Enter}");
    expect(
      await screen.findByText("That is wrong. 4 tries left before a short lock."),
    ).toBeVisible();

    await userEvent.type(screen.getByLabelText("PIN"), "1234{Enter}");

    expect(await screen.findByRole("navigation", { name: "Main menu" })).toBeVisible();
    expect(screen.getByText("Logged in: Rina Akter (Cashier)")).toBeVisible();
    // The menu hides pages the cashier's role can't open. Rust refuses them anyway.
    expect(screen.queryByRole("button", { name: /Users/ })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Backups/ })).toBeVisible();
  });

  it("usr_01: logging out goes back to the login tiles", async () => {
    renderFrame();
    await userEvent.click(await screen.findByRole("button", { name: /Log out/ }));
    expect(await screen.findByRole("heading", { name: "Log in" })).toBeVisible();
    expect(screen.getByRole("button", { name: /Shop Owner/ })).toBeVisible();
  });

  it("usr_03: F12 locks the till and the user's password unlocks it", async () => {
    renderFrame();
    await screen.findByRole("navigation", { name: "Main menu" });
    // user-event's key list has no F12, so the key press is made by hand.
    fireEvent.keyDown(document.body, { key: "F12", code: "F12" });
    fireEvent.keyUp(document.body, { key: "F12", code: "F12" });

    expect(await screen.findByRole("heading", { name: "Till locked" })).toBeVisible();
    expect(screen.queryByRole("navigation", { name: "Main menu" })).not.toBeInTheDocument();
    await userEvent.type(screen.getByLabelText("PIN or password"), "shop-owner-1{Enter}");

    expect(await screen.findByRole("navigation", { name: "Main menu" })).toBeVisible();
  });

  it("usr_03: an admin unlocks to help a cashier", async () => {
    resetFakeShop("set_up", false);
    signInFakeAs("rina");
    renderFrame();
    await userEvent.click(await screen.findByRole("button", { name: /^Lock/ }));
    await screen.findByRole("heading", { name: "Till locked" });

    await userEvent.type(screen.getByLabelText("Admin username"), "owner");
    await userEvent.type(screen.getByLabelText("Admin password"), "shop-owner-1");
    await userEvent.click(screen.getByRole("button", { name: /Unlock as admin/ }));

    expect(await screen.findByText("Logged in: Rina Akter (Cashier)")).toBeVisible();
  });
});
