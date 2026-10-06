import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { allPermissions } from "../../shared/bridge/mock/people";
import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { PurchasingPage } from "./pages/PurchasingPage";

// The purchasing screens against the fake bridge (PUR-01 to PUR-06). The fake shop has
// one supplier, Rahim Traders, and Milk 1L at a cost of 75.00.

function renderPage() {
  render(
    <TooltipProvider>
      <PurchasingPage permissions={allPermissions} />
    </TooltipProvider>,
  );
}

const tab = (name: string) => userEvent.click(screen.getByRole("tab", { name }));

async function pickMilk() {
  await userEvent.type(screen.getByRole("textbox", { name: "Add a product" }), "milk");
  await userEvent.click(await screen.findByRole("button", { name: "Milk 1L" }));
}

// Receives 10 Milk 1L at 75.00 directly, with this invoice total.
async function receiveMilk(invoiceTotal: string) {
  await tab("Receive goods");
  await pickMilk();
  const qty = await screen.findByRole("textbox", { name: "Received of Milk 1L" });
  await userEvent.clear(qty);
  await userEvent.type(qty, "10");
  await userEvent.type(screen.getByRole("textbox", { name: "Cost of Milk 1L" }), "75");
  await userEvent.type(screen.getByRole("textbox", { name: "Invoice total" }), invoiceTotal);
}

describe("PurchasingPage (fake bridge)", () => {
  beforeEach(() => {
    resetFakeShop("set_up", true);
  });

  it("pur_01: adds a supplier and lists it", async () => {
    renderPage();
    expect(await screen.findByRole("button", { name: "Rahim Traders" })).toBeVisible();
    await userEvent.type(screen.getByRole("textbox", { name: "Name" }), "Karim Foods");
    await userEvent.click(screen.getByRole("button", { name: /^Add supplier/ }));
    expect(await screen.findByRole("button", { name: "Karim Foods" })).toBeVisible();
  });

  it("pur_01: a supplier needs a name", async () => {
    renderPage();
    await screen.findByRole("button", { name: "Rahim Traders" });
    await userEvent.click(screen.getByRole("button", { name: /^Add supplier/ }));
    // Shown under the field and in the message box.
    expect(await screen.findAllByText(/Type the supplier's name/)).not.toHaveLength(0);
  });

  it("pur_02: makes an order, saves it and marks it sent", async () => {
    renderPage();
    await tab("Purchase orders");
    expect(await screen.findByText("No purchase orders yet.")).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: /^New order/ }));
    await pickMilk();
    const qty = await screen.findByRole("textbox", { name: "Quantity of Milk 1L" });
    await userEvent.clear(qty);
    await userEvent.type(qty, "24");
    await userEvent.type(screen.getByRole("textbox", { name: "Cost of Milk 1L" }), "75");
    await userEvent.click(screen.getByRole("button", { name: /^Save order/ }));
    expect(await screen.findByText("PO-000001")).toBeVisible();
    expect(screen.getByText("Status: Draft")).toBeVisible();
    expect(screen.getByText("Total ৳1,800.00")).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: /^Mark sent/ }));
    expect(await screen.findByText("Status: Sent")).toBeVisible();
  });

  it("pur_03: receiving shows the lines total and warns when the invoice differs", async () => {
    renderPage();
    await screen.findByRole("button", { name: "Rahim Traders" });
    await receiveMilk("760");
    expect(await screen.findByText("Lines total ৳750.00")).toBeVisible();
    expect(await screen.findByText(/differs from the lines by ৳10.00/)).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: /^Save delivery/ }));
    expect(await screen.findByText(/Delivery GRN-000001 saved\./)).toBeVisible();
  });

  it("pur_05: a payment lowers what the shop owes", async () => {
    renderPage();
    await screen.findByRole("button", { name: "Rahim Traders" });
    await receiveMilk("750");
    await userEvent.click(screen.getByRole("button", { name: /^Save delivery/ }));
    await screen.findByText(/Delivery GRN-000001 saved\./);
    await tab("Suppliers");
    await userEvent.click(await screen.findByRole("button", { name: "Rahim Traders" }));
    expect(await screen.findByText("We owe ৳750.00")).toBeVisible();
    await userEvent.type(screen.getByRole("textbox", { name: "Amount" }), "500");
    await userEvent.click(screen.getByRole("button", { name: /^Record payment/ }));
    expect(await screen.findByText("Payment saved. We now owe ৳250.00.")).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: /^Show statement/ }));
    const statement = (await screen.findByText("Closing balance")).closest("table") as HTMLElement;
    expect(within(statement).getByText("Delivery")).toBeVisible();
    expect(within(statement).getByText("Payment")).toBeVisible();
  });

  it("pur_04: goods sent back lower what the shop owes and are listed", async () => {
    renderPage();
    await tab("Supplier returns");
    expect(await screen.findByText("Nothing sent back yet.")).toBeVisible();
    await pickMilk();
    const qty = await screen.findByRole("textbox", { name: "Sending back of Milk 1L" });
    await userEvent.clear(qty);
    await userEvent.type(qty, "2");
    await userEvent.type(screen.getByRole("textbox", { name: "Reason" }), "Leaking");
    await userEvent.click(screen.getByRole("button", { name: /^Send back/ }));
    expect(
      await screen.findByText("Return SR-000001 saved: ৳150.00 off what we owe."),
    ).toBeVisible();
    expect(await screen.findByRole("cell", { name: "Leaking" })).toBeVisible();
  });

  it("pur_05: what we owe lists suppliers with a balance", async () => {
    renderPage();
    await screen.findByRole("button", { name: "Rahim Traders" });
    await tab("What we owe");
    expect(await screen.findByText("The shop owes no supplier anything.")).toBeVisible();
    await receiveMilk("750");
    await userEvent.click(screen.getByRole("button", { name: /^Save delivery/ }));
    await screen.findByText(/Delivery GRN-000001 saved\./);
    await tab("What we owe");
    const row = (await screen.findByRole("cell", { name: "Rahim Traders" })).closest(
      "tr",
    ) as HTMLElement;
    expect(within(row).getAllByText("৳750.00")).toHaveLength(2);
  });
});
