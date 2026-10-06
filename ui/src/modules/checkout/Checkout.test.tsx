import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { currentSession } from "../../shared/bridge/mock/people";
import { resetFakeShop, shopHandlers } from "../../shared/bridge/mock/shop";
import { tillHandlers } from "../../shared/bridge/mock/till";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { CheckoutPage } from "./pages/CheckoutPage";

const scanBox = () => screen.getByRole("textbox", { name: "Scan or search" });

function renderTill(overrides: { onOpenShift?: () => void } = {}) {
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
        onOpenShift={overrides.onOpenShift ?? vi.fn()}
        onForceClose={vi.fn()}
        onShiftMenu={vi.fn()}
        onLock={vi.fn()}
        onBackOffice={vi.fn()}
      />
    </TooltipProvider>,
  );
}

async function scan(code: string) {
  await userEvent.type(scanBox(), `${code}{Enter}`);
}

const cartRow = (name: string) => screen.getByText(name).closest("tr") as HTMLElement;

describe("CheckoutPage (fake bridge)", () => {
  beforeEach(() => {
    resetFakeShop("set_up", true);
    tillHandlers.shift_open({ float: 0, counts: [] });
  });

  it("chk_20: without a shift the till asks to open one", async () => {
    resetFakeShop("set_up", true);
    const onOpenShift = vi.fn();
    renderTill({ onOpenShift });
    expect(await screen.findByText("Open a shift to start selling")).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: /Open shift/ }));
    expect(onOpenShift).toHaveBeenCalled();
  });

  it("chk_01 chk_18: a scan adds the line and the total comes from the cart", async () => {
    renderTill();
    await screen.findByText("Scan an item to start the sale.");
    expect(scanBox()).toHaveFocus();
    await scan("8901234567890");
    const milk = cartRow("Milk 1L");
    expect(within(milk).getAllByText("৳90.00")).toHaveLength(2);
    await scan("8901234567890");
    expect(within(cartRow("Milk 1L")).getByText("2")).toBeVisible();
    expect(within(screen.getByLabelText("TOTAL")).getByText("৳180.00")).toBeVisible();
    expect(scanBox()).toHaveValue("");
  });

  it("chk_02: an unknown code shows a red message and changes nothing", async () => {
    renderTill();
    await screen.findByText("Scan an item to start the sale.");
    await scan("123");
    expect(await screen.findByText("No product with code 123.")).toBeVisible();
    expect(screen.getByText("Scan an item to start the sale.")).toBeVisible();
  });

  it("chk_03: typed words open the search list and Enter adds the product", async () => {
    renderTill();
    await screen.findByText("Scan an item to start the sale.");
    await userEvent.type(scanBox(), "brea");
    const list = await screen.findByRole("list", { name: "Products" });
    expect(within(list).getByText("Bread")).toBeVisible();
    await userEvent.keyboard("{Enter}");
    expect(await screen.findByText("Bread")).toBeVisible();
  });

  it("chk_04 chk_05: F4 changes the quantity, Del removes, F9 cancels after asking", async () => {
    renderTill();
    await screen.findByText("Scan an item to start the sale.");
    await scan("8901234567890");
    await userEvent.keyboard("[F4]");
    const qty = await screen.findByRole("textbox", { name: "Quantity" });
    await userEvent.clear(qty);
    await userEvent.type(qty, "1.5{Enter}");
    expect(await screen.findByText(/can't be sold in parts/)).toBeVisible();
    await userEvent.keyboard("[F4]");
    const again = await screen.findByRole("textbox", { name: "Quantity" });
    await userEvent.clear(again);
    await userEvent.type(again, "3{Enter}");
    expect(await screen.findByText("৳270.00", { selector: "td" })).toBeVisible();
    await scan("8902222222227");
    await userEvent.keyboard("{Delete}");
    expect(screen.queryByText("Soap")).toBeNull();
    await userEvent.keyboard("[F9]");
    expect(await screen.findByRole("dialog", { name: "Cancel this sale?" })).toBeVisible();
    await userEvent.keyboard("{Enter}");
    expect(await screen.findByText("Scan an item to start the sale.")).toBeVisible();
  });

  it("chk_06: a weighed item opens the weight dialog and M lets the weight be typed", async () => {
    renderTill();
    await screen.findByText("Scan an item to start the sale.");
    await scan("2000000000015");
    expect(await screen.findByRole("dialog", { name: "Weigh Potato" })).toBeVisible();
    await userEvent.keyboard("m");
    await userEvent.type(await screen.findByRole("textbox", { name: "Weight" }), "1.25{Enter}");
    expect(await screen.findByText("1.250")).toBeVisible();
    expect(screen.getByText("৳75.00", { selector: "td" })).toBeVisible();
  });

  it("chk_09 chk_10: F5 holds with a note, F6 lists it and Enter brings it back", async () => {
    renderTill();
    await screen.findByText("Scan an item to start the sale.");
    await scan("8901234567890");
    await userEvent.keyboard("[F5]");
    await userEvent.type(
      await screen.findByRole("textbox", { name: "Note (optional)" }),
      "blue shirt{Enter}",
    );
    expect(await screen.findByText("Scan an item to start the sale.")).toBeVisible();
    expect(screen.getByRole("button", { name: /Held \(1\)/ })).toBeVisible();
    await userEvent.keyboard("[F6]");
    const dialog = await screen.findByRole("dialog", { name: "Sales on hold" });
    expect(within(dialog).getByText("blue shirt")).toBeVisible();
    await userEvent.keyboard("{Enter}");
    expect(await screen.findByText("Milk 1L")).toBeVisible();
  });

  it("pay_01: F10 opens payment with the cash due, and change shows after completing", async () => {
    renderTill();
    await screen.findByText("Scan an item to start the sale.");
    await scan("8901234567890");
    await userEvent.keyboard("[F10]");
    const dialog = await screen.findByRole("dialog", { name: "Payment" });
    const cash = within(dialog).getByRole("textbox", { name: "Cash" });
    expect(cash).toHaveValue("90.00");
    await userEvent.clear(cash);
    await userEvent.type(cash, "100");
    expect(await within(dialog).findByText("৳10.00")).toBeVisible();
    await userEvent.keyboard("{Enter}");
    expect(await screen.findByText("Sale complete · Invoice T1-000001")).toBeVisible();
    expect(screen.getByText("৳10.00")).toBeVisible();
    expect(screen.getByText("Scan an item to start the sale.")).toBeVisible();
  });

  it("pay_02: a method that needs a reference refuses to complete without one", async () => {
    renderTill();
    await screen.findByText("Scan an item to start the sale.");
    await scan("8901234567890");
    await userEvent.keyboard("[F10]");
    const dialog = await screen.findByRole("dialog", { name: "Payment" });
    const cash = within(dialog).getByRole("textbox", { name: "Cash" });
    await userEvent.clear(cash);
    await userEvent.type(within(dialog).getByRole("textbox", { name: "Card" }), "90");
    await userEvent.keyboard("{Enter}");
    expect(await within(dialog).findByText(/Type the reference/)).toBeVisible();
  });

  async function sellMilkForCash() {
    await scan("8901234567890");
    await userEvent.keyboard("[F10]");
    await screen.findByRole("dialog", { name: "Payment" });
    await userEvent.keyboard("{Enter}");
    await screen.findByText("Sale complete · Invoice T1-000001");
  }

  it("pay_09: a printer that is offline keeps the sale and offers Retry and the system printer", async () => {
    const hardware = shopHandlers.settings_get().hardware;
    shopHandlers.settings_save({
      change: {
        section: "hardware",
        value: { ...hardware, receipt_printer: { kind: "tcp", host: "10.0.0.9", port: 9100 } },
      },
    });
    renderTill();
    await screen.findByText("Scan an item to start the sale.");
    await sellMilkForCash();
    expect(await screen.findByText("Receipt not printed. The sale is saved.")).toBeVisible();
    expect(screen.getByRole("button", { name: /Print to system printer/ })).toBeVisible();
    // Still offline: Retry keeps the banner.
    await userEvent.click(screen.getByRole("button", { name: /Retry/ }));
    expect(await screen.findByText("Receipt not printed. The sale is saved.")).toBeVisible();
    // Back on the Preview printer, Retry prints and the banner goes.
    shopHandlers.settings_save({ change: { section: "hardware", value: hardware } });
    await userEvent.click(screen.getByRole("button", { name: /Retry/ }));
    await vi.waitFor(() =>
      expect(screen.queryByText("Receipt not printed. The sale is saved.")).toBeNull(),
    );
  });

  it("pay_10: Ctrl+P reprints the last receipt, marked COPY", async () => {
    renderTill();
    await screen.findByText("Scan an item to start the sale.");
    await userEvent.keyboard("{Control>}p{/Control}");
    expect(
      await screen.findByText("No sale on this till yet, so there is no receipt to reprint."),
    ).toBeVisible();
    await sellMilkForCash();
    expect(screen.queryByText("Receipt not printed. The sale is saved.")).toBeNull();
    await userEvent.keyboard("{Control>}p{/Control}");
    expect(
      await screen.findByText("The last receipt was printed again, marked COPY."),
    ).toBeVisible();
  });

  it("pay_10: Old receipt reprints by invoice number and names an unknown one", async () => {
    renderTill();
    await screen.findByText("Scan an item to start the sale.");
    await sellMilkForCash();
    await userEvent.click(screen.getByRole("button", { name: /Old receipt/ }));
    const dialog = await screen.findByRole("dialog", { name: "Reprint a receipt" });
    await userEvent.type(within(dialog).getByRole("textbox"), "T1-000009{Enter}");
    expect(await screen.findByText("No sale with invoice number T1-000009.")).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: /Old receipt/ }));
    const again = await screen.findByRole("dialog", { name: "Reprint a receipt" });
    await userEvent.type(within(again).getByRole("textbox"), "T1-000001{Enter}");
    expect(
      await screen.findByText("Receipt T1-000001 was printed again, marked COPY."),
    ).toBeVisible();
  });
});
