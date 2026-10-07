import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";

import { resetFakeShop } from "../../shared/bridge/mock/shop";
import { TooltipProvider } from "../../shared/ui/tooltip";
import { LabelsPage } from "./pages/LabelsPage";

// Labels and shelf tags against the fake bridge (CAT-12, CAT-13). The pictures are
// fixed demo PNGs; Rust's golden tests check the real drawings.

function renderPage() {
  render(
    <TooltipProvider>
      <LabelsPage />
    </TooltipProvider>,
  );
}

const button = (name: RegExp) => screen.getByRole("button", { name });

describe("LabelsPage (fake bridge)", () => {
  beforeEach(() => {
    resetFakeShop("set_up", true);
  });

  it("cat_12: a found product is added, previewed and printed", async () => {
    renderPage();
    expect(button(/^Print$/)).toHaveAttribute("aria-disabled", "true");
    await userEvent.type(screen.getByRole("textbox", { name: "Find product" }), "milk");
    await userEvent.click(button(/^Find$/));
    await userEvent.click(await screen.findByRole("button", { name: /^Add Milk/ }));
    expect(await screen.findByRole("cell", { name: /^Milk/ })).toBeVisible();

    const copies = screen.getByRole("textbox", { name: "How many" });
    await userEvent.clear(copies);
    await userEvent.type(copies, "3");
    await userEvent.click(button(/^Preview$/));
    expect(await screen.findByText("× 3")).toBeVisible();
    await userEvent.click(button(/^Print$/));
    expect(
      await screen.findByText(
        "No label printer is set up, so 3 labels went to the Preview printer.",
      ),
    ).toBeVisible();
  });

  it("cat_12: how many must be a whole number from 1 to 500", async () => {
    renderPage();
    await userEvent.type(screen.getByRole("textbox", { name: "Find product" }), "milk");
    await userEvent.click(button(/^Find$/));
    await userEvent.click(await screen.findByRole("button", { name: /^Add Milk/ }));
    const copies = await screen.findByRole("textbox", { name: "How many" });
    await userEvent.clear(copies);
    await userEvent.type(copies, "0");
    await userEvent.click(button(/^Preview$/));
    expect(
      await screen.findByText("Type how many labels as a whole number from 1 to 500."),
    ).toBeVisible();
  });

  it("cat_13: shelf tags on A4 show a sheet to print", async () => {
    renderPage();
    await userEvent.type(
      screen.getByRole("textbox", { name: "Prices changed since" }),
      "2026-03-01",
    );
    await userEvent.click(button(/^Add changed/));
    await screen.findByRole("cell", { name: /^Milk/ });
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Kind" }), "shelf_tag");
    expect(screen.getByRole("combobox", { name: "Size" })).toHaveValue("80x40");
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Print on" }), "a4");
    await userEvent.click(button(/^Print$/));
    expect(await screen.findByText("Press Preview first.")).toBeVisible();
    await userEvent.click(button(/^Preview$/));
    expect(await screen.findByRole("region", { name: "A4 sheet" })).toBeVisible();
  });
});
