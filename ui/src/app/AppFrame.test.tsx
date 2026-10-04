import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { TooltipProvider } from "../shared/ui/tooltip";
import { AppFrame } from "./AppFrame";

describe("AppFrame", () => {
  it("shows the app name and asks Rust (the fake bridge) for the version", async () => {
    render(
      <TooltipProvider>
        <AppFrame />
      </TooltipProvider>,
    );
    expect(screen.getByRole("heading", { name: "Supershop POS" })).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: /Check app/ }));

    expect(await screen.findByText("0.1.0 (fake bridge)")).toBeInTheDocument();
  });
});
