import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { fakeCatalog } from "../../shared/bridge/mock/catalog";
import { customerHandlers } from "../../shared/bridge/mock/customers";
import { currentSession, signInFakeAs } from "../../shared/bridge/mock/people";
import { resetFakeShop, shopHandlers } from "../../shared/bridge/mock/shop";
import { tillHandlers } from "../../shared/bridge/mock/till";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { CheckoutPage } from "./pages/CheckoutPage";

// Discounts, typed prices, customers, price lists and paying from the customer's
// account at the till (C-M8), against the fake bridge.

const scanBox = () => screen.getByRole("textbox", { name: "Scan or search" });

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

async function tillWithMilk() {
  renderTill();
  await screen.findByText("Scan an item to start the sale.");
  await userEvent.type(scanBox(), "8901234567890{Enter}");
  await screen.findByText("Milk 1L");
}

const cartRow = (name: string) => screen.getByText(name).closest("tr") as HTMLElement;
const total = () => within(screen.getByLabelText("TOTAL"));

describe("Checkout pricing and customers (fake bridge)", () => {
  beforeEach(() => {
    resetFakeShop("set_up", true);
  });

  it("chk_12: F7 gives the selected line a discount", async () => {
    tillHandlers.shift_open({ float: 0, counts: [] });
    await tillWithMilk();
    await userEvent.keyboard("[F7]");
    const dialog = await screen.findByRole("dialog", { name: "Discount on Milk 1L" });
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Percent off" }), "10{Enter}");
    expect(await within(cartRow("Milk 1L")).findByText("−৳9.00")).toBeVisible();
    expect(total().getByText("৳81.00")).toBeVisible();
  });

  it("prc_07: a cashier's discount above the limit needs an admin, then works", async () => {
    signInFakeAs("rina");
    tillHandlers.shift_open({ float: 0, counts: [] });
    await tillWithMilk();
    await userEvent.keyboard("{Shift>}[F7]{/Shift}");
    const dialog = await screen.findByRole("dialog", { name: "Discount on the whole sale" });
    expect(within(dialog).getByText("Your limit is 0%.")).toBeVisible();
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Percent off" }), "10{Enter}");
    const approval = await screen.findByRole("dialog", { name: /This needs an admin/ });
    await userEvent.type(within(approval).getByLabelText("Admin password"), "shop-owner-1{Enter}");
    expect(await screen.findByText("Sale discount 10%")).toBeVisible();
    expect(total().getByText("৳81.00")).toBeVisible();
  });

  it("chk_13: F8 types a price and the line is marked", async () => {
    tillHandlers.shift_open({ float: 0, counts: [] });
    await tillWithMilk();
    await userEvent.keyboard("[F8]");
    const dialog = await screen.findByRole("dialog", { name: "Price for Milk 1L" });
    await userEvent.type(
      within(dialog).getByRole("textbox", { name: "New price for one" }),
      "85{Enter}",
    );
    expect(await within(cartRow("Milk 1L")).findByText("Price typed")).toBeVisible();
    expect(total().getByText("৳85.00")).toBeVisible();
  });

  it("chk_11: F3 finds a customer and their tier discount applies at once", async () => {
    tillHandlers.shift_open({ float: 0, counts: [] });
    await tillWithMilk();
    expect(screen.getByText("No customer")).toBeVisible();
    await userEvent.keyboard("[F3]");
    const dialog = await screen.findByRole("dialog", { name: "Customer" });
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Phone or name" }), "1711");
    expect(await within(dialog).findByText(/Asha Begum/)).toBeVisible();
    await userEvent.keyboard("{Enter}");
    expect(await screen.findByText("Asha Begum · +8801711123456")).toBeVisible();
    expect(screen.getByText("Gold price")).toBeVisible();
    expect(screen.getByText("120 points")).toBeVisible();
    expect(within(cartRow("Milk 1L")).getByText("−৳4.50")).toBeVisible();
  });

  it("cus_02: a new customer is added with a phone and a name", async () => {
    tillHandlers.shift_open({ float: 0, counts: [] });
    await tillWithMilk();
    await userEvent.click(screen.getByRole("button", { name: /Customer/ }));
    const dialog = await screen.findByRole("dialog", { name: "Customer" });
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Name" }), "Karim");
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Mobile" }), "123");
    await userEvent.click(within(dialog).getByRole("button", { name: /Add customer/ }));
    expect(
      await within(dialog).findByText("Type a mobile number. Example: 01711 123456"),
    ).toBeVisible();
    const phone = within(dialog).getByRole("textbox", { name: "Mobile" });
    await userEvent.clear(phone);
    await userEvent.type(phone, "01912 345678");
    await userEvent.click(within(dialog).getByRole("button", { name: /Add customer/ }));
    expect(await screen.findByText("Karim · +8801912345678")).toBeVisible();
  });

  it("chk_17: switching to the wholesale list reprices the cart", async () => {
    const milk = fakeCatalog().products.find((each) => each.name === "Milk 1L");
    customerHandlers.price_list_set_items({
      id: "01FAKELIST0000000000000001",
      items: [{ product_id: milk?.id ?? "", price: 8000 }],
    });
    tillHandlers.shift_open({ float: 0, counts: [] });
    await tillWithMilk();
    await userEvent.click(screen.getByRole("button", { name: /Price list/ }));
    const dialog = await screen.findByRole("dialog", { name: "Price list" });
    await userEvent.click(within(dialog).getByRole("button", { name: /Wholesale/ }));
    expect(await screen.findByText("Wholesale prices")).toBeVisible();
    expect(total().getByText("৳80.00")).toBeVisible();
  });

  it("pay_05: points pay up to what the customer has, and the sale says what was earned", async () => {
    shopHandlers.settings_save({
      change: {
        section: "loyalty",
        value: {
          enabled: true,
          spend_unit_minor: 1000,
          points_per_unit: 1,
          value_per_point_minor: 10,
          min_redeem: 0,
        },
      },
    });
    tillHandlers.shift_open({ float: 0, counts: [] });
    await tillWithMilk();
    tillHandlers.checkout_set_customer({ customerId: "01FAKECUST0000000000000001" });
    await userEvent.keyboard("[F10]");
    const dialog = await screen.findByRole("dialog", { name: "Payment" });
    expect(await within(dialog).findByText("120 points can pay up to ৳12.00")).toBeVisible();
    await userEvent.click(within(dialog).getByRole("button", { name: "Use points" }));
    await userEvent.click(within(dialog).getByRole("button", { name: /Complete/ }));
    expect(await screen.findByText(/points earned/)).toBeVisible();
  });
});
