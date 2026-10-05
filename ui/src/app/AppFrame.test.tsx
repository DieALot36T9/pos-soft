import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

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

  it("after setup, the admin signs in before the back office opens", async () => {
    resetFakeShop("set_up", false);
    renderFrame();
    await userEvent.type(await screen.findByRole("textbox", { name: "Username" }), "owner");
    await userEvent.type(screen.getByLabelText("Password"), "wrong-password");
    await userEvent.click(screen.getByRole("button", { name: /Sign in/ }));
    expect(await screen.findByText("The username or password is wrong.")).toBeVisible();

    await userEvent.clear(screen.getByLabelText("Password"));
    await userEvent.type(screen.getByLabelText("Password"), "shop-owner-1{Enter}");

    expect(await screen.findByRole("navigation", { name: "Main menu" })).toBeVisible();
    expect(screen.getByText("Signed in: Shop Owner")).toBeVisible();
  });

  it("signing out goes back to sign-in", async () => {
    renderFrame();
    await userEvent.click(await screen.findByRole("button", { name: /Sign out/ }));
    expect(await screen.findByRole("heading", { name: "Sign in" })).toBeVisible();
  });
});
