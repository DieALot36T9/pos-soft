import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { currentSession, signInFakeAs } from "../../shared/bridge/mock/people";
import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { tillHandlers } from "../../shared/bridge/mock/till";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { CheckoutPage } from "./pages/CheckoutPage";

// Returns, exchanges and voids at the till (C-M9, RET-01 to RET-05), against the fake bridge.

const CASH = "00000000000000000000000001";

// One Milk 1L (90.00) sold for cash before the till screen opens: invoice T1-000001.
function sellMilk() {
  tillHandlers.shift_open({ float: 0, counts: [] });
  tillHandlers.checkout_scan({ input: "8901234567890" });
  tillHandlers.checkout_complete({
    payments: [{ method_id: CASH, amount: 9000, reference: null }],
    approval: null,
  });
}

function renderTill() {
  const session = currentSession();
  if (session === null) {
    throw new Error("the fake shop has nobody signed in");
  }
  render(
    <TooltipProvider>
      <CheckoutPage
        session={session}
        reloadKey={0}
        justLoggedIn={false}
        onOpenShift={vi.fn()}
        onForceClose={vi.fn()}
        onShiftMenu={vi.fn()}
        onLock={vi.fn()}
        onBackOffice={vi.fn()}
      />
    </TooltipProvider>,
  );
}

// Opens the return dialog with Ctrl+R and finds the milk sale.
async function openSale() {
  renderTill();
  await screen.findByText("Scan an item to start the sale.");
  await userEvent.keyboard("{Control>}r{/Control}");
  const dialog = await screen.findByRole("dialog", { name: "Return, exchange or void" });
  await userEvent.type(
    within(dialog).getByRole("textbox", { name: "Invoice number" }),
    "T1-000001{Enter}",
  );
  await within(dialog).findByText(/Invoice T1-000001/);
  return dialog;
}

async function oneMilkBack(dialog: HTMLElement) {
  await userEvent.type(
    within(dialog).getByRole("textbox", { name: "Coming back from Milk 1L" }),
    "1",
  );
  await userEvent.type(within(dialog).getByRole("textbox", { name: "Reason" }), "Wrong size");
  expect(await within(dialog).findAllByText("৳90.00")).not.toHaveLength(0);
}

describe("Returns, exchanges and voids (fake bridge)", () => {
  beforeEach(() => {
    resetFakeShop("set_up", true);
  });

  it("ret_01: a return with the receipt refunds in cash and says how much to give back", async () => {
    sellMilk();
    const dialog = await openSale();
    await oneMilkBack(dialog);
    await userEvent.click(within(dialog).getByRole("button", { name: /^Refund/ }));
    expect(await screen.findByText("Return T1-R000001 saved")).toBeVisible();
    const bar = screen.getByRole("status");
    expect(within(bar).getByText("Give back")).toBeVisible();
    expect(within(bar).getByText("৳90.00")).toBeVisible();
  });

  it("ret_01: a cashier's return needs an admin, then goes through", async () => {
    signInFakeAs("rina");
    sellMilk();
    const dialog = await openSale();
    await oneMilkBack(dialog);
    await userEvent.click(within(dialog).getByRole("button", { name: /^Refund/ }));
    const approval = await screen.findByRole("dialog", { name: /This needs an admin/ });
    await userEvent.type(within(approval).getByLabelText("Admin password"), "shop-owner-1{Enter}");
    expect(await screen.findByText("Return T1-R000001 saved")).toBeVisible();
  });

  it("ret_04: a sale from this shift is voided with a reason", async () => {
    sellMilk();
    const dialog = await openSale();
    await userEvent.click(within(dialog).getByRole("button", { name: /^Void sale/ }));
    const confirm = await screen.findByRole("dialog", { name: "Void sale T1-000001?" });
    await userEvent.type(within(confirm).getByRole("textbox", { name: "Reason" }), "Rang up twice");
    await userEvent.click(within(confirm).getByRole("button", { name: /^Void sale/ }));
    expect(await screen.findByText("Sale T1-000001 voided")).toBeVisible();
  });

  it("ret_05: an exchange puts the credit on the cart, and it can go back in cash", async () => {
    sellMilk();
    const dialog = await openSale();
    await oneMilkBack(dialog);
    await userEvent.click(within(dialog).getByRole("button", { name: /^Exchange/ }));
    expect(await screen.findByText("Exchange from T1-R000001: ৳90.00 to spend")).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: /^Give credit back/ }));
    expect(await screen.findByText("Exchange credit given back")).toBeVisible();
    expect(screen.queryByText(/Exchange from T1-R000001/)).toBeNull();
  });

  it("ret_03: without a receipt, products are picked and priced at today's price", async () => {
    tillHandlers.shift_open({ float: 0, counts: [] });
    renderTill();
    await screen.findByText("Scan an item to start the sale.");
    await userEvent.click(screen.getByRole("button", { name: /^Return/ }));
    const dialog = await screen.findByRole("dialog", { name: "Return, exchange or void" });
    await userEvent.click(within(dialog).getByRole("button", { name: /^No receipt/ }));
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Product" }), "Milk");
    await userEvent.click(await within(dialog).findByRole("button", { name: /Add Milk 1L/ }));
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Reason" }), "No receipt");
    expect(await within(dialog).findAllByText("৳90.00")).not.toHaveLength(0);
    await userEvent.click(within(dialog).getByRole("button", { name: /^Refund/ }));
    expect(await screen.findByText("Return T1-R000001 saved")).toBeVisible();
  });

  it("ret_01: returning more than is left says how many may come back", async () => {
    sellMilk();
    const dialog = await openSale();
    await userEvent.type(
      within(dialog).getByRole("textbox", { name: "Coming back from Milk 1L" }),
      "3",
    );
    expect(
      await within(dialog).findByText("Only 1 can still come back on this line."),
    ).toBeVisible();
  });
});
