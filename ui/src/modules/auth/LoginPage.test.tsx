import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { LoginPage } from "./pages/LoginPage";
import { MyAccountPage } from "./pages/MyAccountPage";

function renderLogin(onLoggedIn = vi.fn()) {
  render(
    <TooltipProvider>
      <LoginPage onLoggedIn={onLoggedIn} />
    </TooltipProvider>,
  );
  return onLoggedIn;
}

describe("Login (fake bridge)", () => {
  beforeEach(() => resetFakeShop("set_up", false));

  it("usr_01: shows a tile for each active user and asks the right secret", async () => {
    renderLogin();
    await userEvent.click(await screen.findByRole("button", { name: /Shop Owner/ }));
    expect(screen.getByRole("heading", { name: "Hello, Shop Owner" })).toBeVisible();
    expect(screen.getByLabelText("Password")).toBeVisible();

    await userEvent.click(screen.getByRole("button", { name: /Back/ }));
    await userEvent.click(screen.getByRole("button", { name: /Rina Akter/ }));
    expect(screen.getByLabelText("PIN")).toBeVisible();
  });

  it("usr_01: a correct PIN logs in", async () => {
    const onLoggedIn = renderLogin();
    await userEvent.click(await screen.findByRole("button", { name: /Rina Akter/ }));
    await userEvent.type(screen.getByLabelText("PIN"), "1234");
    await userEvent.click(screen.getByRole("button", { name: /^Log in/ }));
    await vi.waitFor(() => expect(onLoggedIn).toHaveBeenCalled());
    expect(onLoggedIn.mock.calls[0]?.[0]).toMatchObject({ username: "rina", locked: false });
  });

  it("usr_02: the fifth wrong PIN locks the user with a countdown", async () => {
    renderLogin();
    await userEvent.click(await screen.findByRole("button", { name: /Rina Akter/ }));
    for (let attempt = 0; attempt < 4; attempt += 1) {
      await userEvent.type(screen.getByLabelText("PIN"), "9999{Enter}");
    }
    expect(await screen.findByText(/1 tries left/)).toBeVisible();
    await userEvent.type(screen.getByLabelText("PIN"), "9999{Enter}");
    expect(await screen.findByText(/Try again in 60 seconds/)).toBeVisible();
  });

  it("usr_09: the recovery code sets a new password and shows a new code once", async () => {
    renderLogin();
    await userEvent.click(await screen.findByRole("button", { name: /Forgot admin password/ }));
    await userEvent.type(screen.getByLabelText("Recovery code"), "AAAA-AAAA-AAAA-AAAA-AAAA");
    await userEvent.type(screen.getByLabelText("Admin username"), "owner");
    await userEvent.type(screen.getByLabelText("New password"), "new-owner-pass");
    await userEvent.type(screen.getByLabelText("New password again"), "new-owner-pass");
    await userEvent.click(screen.getByRole("button", { name: /Set password/ }));
    expect(await screen.findByText(/The recovery code is wrong/)).toBeVisible();

    await userEvent.clear(screen.getByLabelText("Recovery code"));
    await userEvent.type(screen.getByLabelText("Recovery code"), "7kq2 m9xd 4rtw pz8c h3nb");
    await userEvent.click(screen.getByRole("button", { name: /Set password/ }));

    expect(await screen.findByRole("heading", { name: "The new password is set" })).toBeVisible();
    expect(screen.getByText("H3NB-PZ8C-4RTW-M9XD-7KQ2")).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: /I wrote it down/ }));
    expect(await screen.findByRole("heading", { name: "Log in" })).toBeVisible();
  });
});

describe("My PIN and password (fake bridge)", () => {
  beforeEach(() => resetFakeShop("set_up", true));

  it("usr_07: the current password is checked before the new one is saved", async () => {
    render(
      <TooltipProvider>
        <MyAccountPage />
      </TooltipProvider>,
    );
    await userEvent.type(screen.getByLabelText("Current PIN or password"), "wrong-pass");
    await userEvent.type(screen.getByLabelText("New PIN"), "4821");
    await userEvent.type(screen.getByLabelText("New one again"), "4821");
    await userEvent.click(screen.getByRole("button", { name: /^Save/ }));
    expect(await screen.findByText(/The current PIN or password is wrong/)).toBeVisible();

    await userEvent.type(screen.getByLabelText("Current PIN or password"), "shop-owner-1");
    await userEvent.click(screen.getByRole("button", { name: /^Save/ }));
    expect(await screen.findByText(/Saved. Use the new one/)).toBeVisible();
  });

  it("usr_07: the two new ones must match", async () => {
    render(
      <TooltipProvider>
        <MyAccountPage />
      </TooltipProvider>,
    );
    await userEvent.type(screen.getByLabelText("New PIN"), "4821");
    await userEvent.type(screen.getByLabelText("New one again"), "4822");
    await userEvent.click(screen.getByRole("button", { name: /^Save/ }));
    expect(await screen.findByText(/The two new ones are different/)).toBeVisible();
  });
});
