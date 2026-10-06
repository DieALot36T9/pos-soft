import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { RolesPage } from "./pages/RolesPage";
import { UsersPage } from "./pages/UsersPage";

function renderPage(page: "users" | "roles") {
  render(<TooltipProvider>{page === "users" ? <UsersPage /> : <RolesPage />}</TooltipProvider>);
}

describe("Users (fake bridge)", () => {
  beforeEach(() => resetFakeShop("set_up", true));

  it("usr_06: an admin adds a cashier with a PIN", async () => {
    renderPage("users");
    await screen.findByRole("cell", { name: "Rina Akter" });
    await userEvent.type(screen.getByLabelText("Username"), "karim");
    await userEvent.type(screen.getByLabelText("Name on screen"), "Karim Hossain");
    await userEvent.type(screen.getByLabelText("PIN"), "12");
    await userEvent.click(screen.getByRole("button", { name: /Save user/ }));
    expect(await screen.findByText(/Use 4 to 8 digits for the PIN/)).toBeVisible();

    await userEvent.clear(screen.getByLabelText("PIN"));
    await userEvent.type(screen.getByLabelText("PIN"), "2468");
    await userEvent.click(screen.getByRole("button", { name: /Save user/ }));

    const row = (await screen.findByRole("cell", { name: "Karim Hossain" })).closest("tr");
    expect(row).not.toBeNull();
    expect(within(row as HTMLElement).getByText("Cashier")).toBeVisible();
    expect(within(row as HTMLElement).getByText("PIN")).toBeVisible();
  });

  it("usr_06: the last active admin can't be disabled", async () => {
    renderPage("users");
    const row = (await screen.findByRole("cell", { name: "Shop Owner" })).closest("tr");
    await userEvent.click(within(row as HTMLElement).getByRole("button"));
    expect(screen.getByRole("heading", { name: "Change Shop Owner" })).toBeVisible();
    await userEvent.click(screen.getByLabelText("Can log in"));
    await userEvent.click(screen.getByRole("button", { name: /Save user/ }));
    expect(await screen.findByText(/This is the only active admin/)).toBeVisible();
  });

  it("usr_06: disabling a cashier keeps them in the list", async () => {
    renderPage("users");
    const row = (await screen.findByRole("cell", { name: "Rina Akter" })).closest("tr");
    await userEvent.click(within(row as HTMLElement).getByRole("button"));
    await userEvent.click(screen.getByLabelText("Can log in"));
    await userEvent.click(screen.getByRole("button", { name: /Save user/ }));
    const after = (await screen.findByRole("cell", { name: "Rina Akter" })).closest("tr");
    expect(await within(after as HTMLElement).findByText("Disabled")).toBeVisible();
  });
});

describe("Roles (fake bridge)", () => {
  beforeEach(() => resetFakeShop("set_up", true));

  it("usr_04: an admin adds a Supervisor who may void sales and give 10%", async () => {
    renderPage("roles");
    await screen.findByRole("cell", { name: "Cashier" });
    await userEvent.type(screen.getByLabelText("Role name"), "Supervisor");
    await userEvent.clear(screen.getByLabelText("Biggest discount (%)"));
    await userEvent.type(screen.getByLabelText("Biggest discount (%)"), "10");
    await userEvent.click(screen.getByLabelText("Void a completed sale"));
    await userEvent.click(screen.getByRole("button", { name: /Save role/ }));

    const row = (await screen.findByRole("cell", { name: "Supervisor" })).closest("tr");
    expect(within(row as HTMLElement).getByText("1 of 42")).toBeVisible();
    expect(within(row as HTMLElement).getByText("10%")).toBeVisible();
  });

  it("usr_04: the Admin role has every permission and no edit button", async () => {
    renderPage("roles");
    const row = (await screen.findByRole("cell", { name: "Admin" })).closest("tr");
    expect(within(row as HTMLElement).getByText("42 of 42")).toBeVisible();
    expect(within(row as HTMLElement).queryByRole("button")).not.toBeInTheDocument();
  });
});
