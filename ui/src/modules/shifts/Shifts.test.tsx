import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { signInFakeAs } from "../../shared/bridge/mock/people";
import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { tillHandlers } from "../../shared/bridge/mock/till";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { CloseShiftDialog } from "./pages/CloseShiftDialog";
import { OpenShiftDialog } from "./pages/OpenShiftDialog";
import { ShiftMenu } from "./pages/ShiftMenu";

const wrap = (node: React.ReactNode) => render(<TooltipProvider>{node}</TooltipProvider>);

describe("shift dialogs (fake bridge)", () => {
  beforeEach(() => resetFakeShop("set_up", true));

  it("shf_01: opens a shift with a float counted note by note", async () => {
    const onOpened = vi.fn();
    wrap(<OpenShiftDialog onOpened={onOpened} onClose={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: /Count by note/ }));
    await userEvent.type(await screen.findByRole("textbox", { name: "৳500.00" }), "2");
    await userEvent.type(screen.getByRole("textbox", { name: "৳100.00" }), "1");
    expect(await screen.findByText("৳1,100.00")).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: /Open shift/ }));
    await vi.waitFor(() => expect(onOpened).toHaveBeenCalled());
    expect(tillHandlers.shift_current().shift?.opening_float).toBe(110000);
  });

  it("shf_03: a cashier's paid-out asks for an admin's approval", async () => {
    signInFakeAs("rina");
    tillHandlers.shift_open({ float: 50000, counts: [] });
    wrap(<ShiftMenu onClosedShift={vi.fn()} onClose={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: /Paid out/ }));
    await userEvent.type(await screen.findByRole("textbox", { name: "Amount" }), "200");
    await userEvent.type(screen.getByRole("textbox", { name: "Reason" }), "Delivery man");
    await userEvent.click(screen.getByRole("button", { name: /Save/ }));
    const approval = await screen.findByRole("dialog", { name: /This needs an admin/ });
    await userEvent.type(within(approval).getByLabelText("Admin password"), "shop-owner-1{Enter}");
    expect(await screen.findByText("Saved.")).toBeVisible();
  });

  it("shf_06: closing shows expected, counted and the shortage", async () => {
    tillHandlers.shift_open({ float: 50000, counts: [] });
    const onClosed = vi.fn();
    wrap(<CloseShiftDialog force={false} onClosed={onClosed} onClose={vi.fn()} />);
    await userEvent.type(await screen.findByRole("textbox", { name: "৳200.00" }), "2");
    await userEvent.click(screen.getByRole("button", { name: /Close shift/ }));
    const summary = await screen.findByRole("dialog", { name: "Shift closed" });
    expect(within(summary).getByText("৳500.00")).toBeVisible();
    expect(within(summary).getByText("-৳100.00 · Short")).toBeVisible();
    await userEvent.click(within(summary).getByRole("button", { name: /Done/ }));
    expect(onClosed).toHaveBeenCalled();
  });

  it("shf_05: the X report prints from the shift menu", async () => {
    tillHandlers.shift_open({ float: 50000, counts: [] });
    wrap(<ShiftMenu onClosedShift={vi.fn()} onClose={vi.fn()} />);
    await userEvent.click(await screen.findByRole("button", { name: /X report/ }));
    expect(await screen.findByText("X report printed.")).toBeVisible();
  });

  it("shf_06: the summary says the Z report printed", async () => {
    tillHandlers.shift_open({ float: 50000, counts: [] });
    wrap(<CloseShiftDialog force={false} onClosed={vi.fn()} onClose={vi.fn()} />);
    await userEvent.type(await screen.findByRole("textbox", { name: "৳500.00" }), "1");
    await userEvent.click(screen.getByRole("button", { name: /Close shift/ }));
    expect(await screen.findByText("Z report printed.")).toBeVisible();
  });
});
