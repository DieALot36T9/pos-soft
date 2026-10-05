import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { SetupWizard } from "./pages/SetupWizard";

const fresh = { completed: [], next_step: "shop" as const, is_complete: false };

function renderWizard(status = fresh) {
  const onFinished = vi.fn();
  render(
    <TooltipProvider>
      <SetupWizard status={status} onFinished={onFinished} />
    </TooltipProvider>,
  );
  return onFinished;
}

const next = () => userEvent.click(screen.getByRole("button", { name: /^Next/ }));

describe("SetupWizard (fake bridge)", () => {
  beforeEach(() => resetFakeShop("fresh"));

  it("set_04: refuses a shop without a name and says so under the field", async () => {
    renderWizard();
    await userEvent.click(screen.getByRole("button", { name: /Set up a new shop/ }));
    await screen.findByRole("heading", { name: "Your shop" });

    await next();

    expect(
      await screen.findAllByText("Type the shop's name. Example: Rahim Store"),
    ).not.toHaveLength(0);
    expect(screen.getByRole("textbox", { name: "Shop name" })).toHaveAttribute(
      "aria-invalid",
      "true",
    );
  });

  it("set_01..set_12: goes through every step to the end", async () => {
    const onFinished = renderWizard();
    await userEvent.click(screen.getByRole("button", { name: /Set up a new shop/ }));
    await userEvent.type(await screen.findByRole("textbox", { name: "Shop name" }), "Rahim Store");
    await next();

    expect(await screen.findByTestId("money-preview")).toHaveTextContent("৳12,34,567.50");
    await next();

    await screen.findByRole("heading", { name: "Tax" });
    await next();
    expect(await screen.findByText("Add at least one tax rate first.")).toBeVisible();
    await userEvent.type(screen.getByRole("textbox", { name: "Rate name" }), "Standard 15%");
    await userEvent.type(screen.getByRole("textbox", { name: "Percent" }), "15");
    await userEvent.click(screen.getByRole("button", { name: /Add rate/ }));
    expect(await screen.findByText("15%")).toBeVisible();
    await next();

    await userEvent.type(await screen.findByRole("textbox", { name: "Username" }), "owner");
    await userEvent.type(screen.getByRole("textbox", { name: "Your name" }), "Shop Owner");
    await userEvent.type(screen.getByLabelText("Password"), "shop-owner-1");
    await userEvent.type(screen.getByLabelText("Password again"), "shop-owner-1");
    await next();
    expect(await screen.findByTestId("recovery-code")).toHaveTextContent(
      "7KQ2-M9XD-4RTW-PZ8C-H3NB",
    );
    await userEvent.click(screen.getByRole("button", { name: /I have saved it/ }));

    await screen.findByRole("heading", { name: "This till" });
    await next();
    await screen.findByRole("heading", { name: "Printer" });
    await userEvent.click(screen.getByRole("button", { name: /Skip for now/ }));
    await screen.findByRole("heading", { name: "Backups" });
    await next();
    await screen.findByRole("heading", { name: "Products" });
    await next();

    expect(await screen.findByRole("heading", { name: "Your shop is ready" })).toBeVisible();
    expect(screen.getByText(/printer isn't set up yet/)).toBeVisible();
    await userEvent.click(screen.getByRole("button", { name: /Start/ }));
    expect(onFinished).toHaveBeenCalledOnce();
  });

  it("set_08: the two passwords must match", async () => {
    renderWizard({
      completed: ["shop", "currency", "tax"],
      next_step: "admin",
      is_complete: false,
    } as never);
    await userEvent.type(await screen.findByRole("textbox", { name: "Username" }), "owner");
    await userEvent.type(screen.getByRole("textbox", { name: "Your name" }), "Owner");
    await userEvent.type(screen.getByLabelText("Password"), "shop-owner-1");
    await userEvent.type(screen.getByLabelText("Password again"), "shop-owner-2");
    await next();
    expect(await screen.findAllByText(/two passwords are different/)).not.toHaveLength(0);
  });

  it("set_02: opens at the first unfinished step with the steps before marked done", async () => {
    renderWizard({
      completed: ["shop", "currency"],
      next_step: "tax",
      is_complete: false,
    } as never);
    expect(await screen.findByRole("heading", { name: "Tax" })).toBeVisible();
    const steps = screen.getByRole("navigation", { name: "Setup steps" });
    expect(steps.querySelector('[aria-current="step"]')).toHaveTextContent("3. Tax");
  });

  it("restores from a backup on the welcome screen", async () => {
    const onFinished = renderWizard();
    await userEvent.click(screen.getByRole("button", { name: /Restore from a backup/ }));
    await vi.waitFor(() => expect(onFinished).toHaveBeenCalledOnce());
  });
});
