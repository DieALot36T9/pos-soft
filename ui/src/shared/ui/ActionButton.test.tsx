import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PauseCircle } from "lucide-react";
import { describe, expect, it, vi } from "vitest";

import { ActionButton } from "./ActionButton";
import { TooltipProvider } from "./tooltip";

function renderButton(props: Partial<Parameters<typeof ActionButton>[0]> = {}) {
  const onClick = vi.fn();
  render(
    <TooltipProvider>
      <ActionButton
        colour="amber"
        icon={PauseCircle}
        label="Hold"
        tip="Put this sale aside and serve the next customer."
        shortcut="F5"
        onClick={onClick}
        {...props}
      />
    </TooltipProvider>,
  );
  return onClick;
}

describe("ActionButton", () => {
  it("shows the label and the shortcut key", () => {
    renderButton();
    expect(screen.getByRole("button", { name: /Hold/ })).toHaveTextContent("HoldF5");
  });

  it("shows its tooltip when reached with the Tab key", async () => {
    renderButton();
    await userEvent.tab();
    expect(await screen.findByRole("tooltip")).toHaveTextContent(
      "Put this sale aside and serve the next customer.",
    );
  });

  it("runs the action when clicked", async () => {
    const onClick = renderButton();
    await userEvent.click(screen.getByRole("button"));
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("does nothing when disabled, and the tooltip says why", async () => {
    const onClick = renderButton({ disabledReason: "Add an item first." });
    await userEvent.tab();
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Add an item first.");
    await userEvent.keyboard("{Enter}");
    await userEvent.click(screen.getByRole("button"));
    expect(onClick).not.toHaveBeenCalled();
  });
});
