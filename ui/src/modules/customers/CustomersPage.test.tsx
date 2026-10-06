import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import type { Permission } from "../../generated/Permission";
import { allPermissions, signInFakeAs } from "../../shared/bridge/mock/people";
import { resetFakeShop, shopHandlers } from "../../shared/bridge/mock/shop";
import { tillHandlers } from "../../shared/bridge/mock/till";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { CustomersPage } from "./pages/CustomersPage";

// The customer screens against the fake bridge (CUS-01 to CUS-05).

function renderPage(permissions: Permission[] = allPermissions) {
  render(
    <TooltipProvider>
      <CustomersPage permissions={permissions} />
    </TooltipProvider>,
  );
}

// Asha buys 300.00 on credit, so she owes something.
function ashaOwes() {
  shopHandlers.settings_save({
    change: { section: "credit", value: { enabled: true, default_limit_minor: 0 } },
  });
  tillHandlers.shift_open({ float: 0, counts: [] });
  tillHandlers.checkout_scan({ input: "8901234567890" });
  tillHandlers.checkout_set_customer({ customerId: "01FAKECUST0000000000000001" });
  const due = tillHandlers.checkout_payment_preview({ payments: [] }).due;
  tillHandlers.checkout_complete({
    payments: [{ method_id: "00000000000000000000000002", amount: due, reference: null }],
  });
  return due;
}

describe("CustomersPage (fake bridge)", () => {
  beforeEach(() => {
    resetFakeShop("set_up", true);
  });

  it("cus_02: lists customers and finds one by part of the phone", async () => {
    renderPage();
    expect(await screen.findByRole("button", { name: "Asha Begum" })).toBeVisible();
    expect(screen.getByRole("button", { name: "Babul Mia" })).toBeVisible();
    await userEvent.type(screen.getByRole("textbox", { name: "Phone or name" }), "1811");
    await waitGone("Asha Begum");
    expect(screen.getByRole("button", { name: "Babul Mia" })).toBeVisible();
  });

  it("cus_01: adds a customer; a phone in use names who has it", async () => {
    renderPage();
    await screen.findByRole("button", { name: "Asha Begum" });
    const form = screen.getByRole("heading", { name: "Add a customer" }).closest("section");
    const add = within(form as HTMLElement);
    await userEvent.type(add.getByRole("textbox", { name: "Name" }), "Karim");
    await userEvent.type(add.getByRole("textbox", { name: "Mobile" }), "01711 123456");
    await userEvent.click(add.getByRole("button", { name: /Add customer/ }));
    expect(
      await add.findByText("This number belongs to Asha Begum. Search for them instead."),
    ).toBeVisible();
    const phone = add.getByRole("textbox", { name: "Mobile" });
    await userEvent.clear(phone);
    await userEvent.type(phone, "01912 345678");
    await userEvent.click(add.getByRole("button", { name: /Add customer/ }));
    expect(await screen.findByRole("button", { name: "Karim" })).toBeVisible();
  });

  it("cus_03: the customer page shows balances, purchases and points", async () => {
    ashaOwes();
    renderPage();
    await userEvent.click(await screen.findByRole("button", { name: "Asha Begum" }));
    expect(await screen.findByRole("heading", { name: "Asha Begum" })).toBeVisible();
    expect(screen.getByText("+8801711123456 · Tier: Gold")).toBeVisible();
    const purchases = screen.getByRole("heading", { name: "Purchases" }).closest("section");
    expect(within(purchases as HTMLElement).getByText("T1-000001")).toBeVisible();
    const points = screen.getByRole("heading", { name: "Points" }).closest("section");
    expect(within(points as HTMLElement).getByText("Earned")).toBeVisible();
  });

  it("cus_05: a debt payment lowers what is owed; more than owed is refused", async () => {
    const owed = ashaOwes();
    renderPage();
    await userEvent.click(await screen.findByRole("button", { name: "Asha Begum" }));
    const form = (await screen.findByRole("heading", { name: "Take a payment" })).closest(
      "section",
    ) as HTMLElement;
    const pay = within(form);
    await userEvent.type(pay.getByRole("textbox", { name: "Amount" }), "9999");
    await userEvent.click(pay.getByRole("button", { name: /Take payment/ }));
    expect(
      await screen.findByText("That is more than the customer owes. Type at most what they owe."),
    ).toBeVisible();
    const amount = pay.getByRole("textbox", { name: "Amount" });
    await userEvent.clear(amount);
    await userEvent.type(amount, "40");
    await userEvent.click(pay.getByRole("button", { name: /Take payment/ }));
    expect(await screen.findByText(/Payment taken/)).toBeVisible();
    expect(
      screen.getByText(`Payment taken. Still owes ৳${((owed - 4000) / 100).toFixed(2)}.`),
    ).toBeVisible();
  });

  it("cus_04: a points correction needs a reason, and the role decides who sees it", async () => {
    renderPage();
    await userEvent.click(await screen.findByRole("button", { name: "Asha Begum" }));
    const form = (await screen.findByRole("heading", { name: "Correct points" })).closest(
      "section",
    ) as HTMLElement;
    const adjust = within(form);
    await userEvent.type(adjust.getByRole("textbox", { name: "Points" }), "50");
    await userEvent.click(adjust.getByRole("button", { name: /Correct points/ }));
    expect(await adjust.findByText("Type a reason. Example: delivery man for rice")).toBeVisible();
    await userEvent.type(adjust.getByRole("textbox", { name: "Reason" }), "Paper card");
    await userEvent.click(adjust.getByRole("button", { name: /Correct points/ }));
    expect(await adjust.findByText("Points now: 170.")).toBeVisible();
  });

  it("cus_01: a cashier sees the page but not the edit, archive or points forms", async () => {
    signInFakeAs("rina");
    renderPage(["customer_view_create", "customer_receive_payment"]);
    await userEvent.click(await screen.findByRole("button", { name: "Asha Begum" }));
    await screen.findByRole("heading", { name: "Asha Begum" });
    expect(screen.queryByRole("heading", { name: "Correct points" })).toBeNull();
    expect(screen.queryByRole("heading", { name: "Details" })).toBeNull();
    expect(screen.queryByRole("button", { name: /Archive/ })).toBeNull();
    expect(screen.getByRole("heading", { name: "Take a payment" })).toBeVisible();
  });
});

// Waits until the customer's name button is gone from the list.
async function waitGone(name: string) {
  await waitFor(() => expect(screen.queryByRole("button", { name })).toBeNull());
}
