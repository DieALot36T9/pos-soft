import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { TooltipProvider } from "../../shared/ui/tooltip";
import { typeLikeScanner } from "./components/typeLikeScanner";
import { HardwarePage } from "./pages/HardwarePage";

function renderPage(devTools = true) {
  render(
    <TooltipProvider>
      <HardwarePage devTools={devTools} />
    </TooltipProvider>,
  );
}

describe("HardwarePage (fake bridge)", () => {
  it("hw_02/hw_10: a test print shows in the Preview printer as a picture", async () => {
    renderPage();
    const preview = screen.getByRole("region", { name: "Preview printer" });
    expect(within(preview).getByText(/Nothing printed yet/)).toBeInTheDocument();

    await userEvent.click(screen.getByRole("radio", { name: "58 mm" }));
    await userEvent.click(screen.getByRole("button", { name: /Test print/ }));

    expect(await screen.findByText("The test receipt was sent to the printer.")).toBeVisible();
    expect(await within(preview).findByRole("img", { name: "Printout 1" })).toBeVisible();
    expect(within(preview).getByText(/test-receipt-mm58\.png/)).toBeInTheDocument();
    expect(within(preview).getByRole("button", { name: /Print on another printer/ })).toBeVisible();
  });

  it("hw_02: the Test print button says what it prints", async () => {
    renderPage();
    screen.getByRole("button", { name: /Test print/ }).focus();
    expect(await screen.findByRole("tooltip")).toHaveTextContent(/currency sign, a logo/);
  });

  it("hw_10: hides the fake scale and scan simulator without the dev tools", () => {
    renderPage(false);
    expect(screen.queryByRole("heading", { name: "Fake scale" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Scan simulator/ })).not.toBeInTheDocument();
  });

  it("hw_10: the fake scale refuses a weight that isn't a number", async () => {
    renderPage();
    const weight = await screen.findByRole("textbox", { name: "Weight" });
    await userEvent.clear(weight);
    await userEvent.type(weight, "1,2x");
    await userEvent.click(screen.getByRole("button", { name: /Set weight/ }));

    expect(await screen.findByText(/at most 3 decimals/)).toBeVisible();
    expect(weight).toHaveAttribute("aria-invalid", "true");
  });

  it("hw_10: reading a moving weight shows Moving and what the scale sent", async () => {
    renderPage();
    const weight = await screen.findByRole("textbox", { name: "Weight" });
    await userEvent.clear(weight);
    await userEvent.type(weight, "1.250");
    await userEvent.click(screen.getByRole("checkbox", { name: "Weight is still" }));
    await userEvent.click(screen.getByRole("button", { name: /Set weight/ }));
    await userEvent.click(screen.getByRole("button", { name: /Read scale/ }));

    expect(await screen.findByText("Moving")).toBeVisible();
    expect(screen.getByText("The scale sent: US 1.250 kg")).toBeVisible();
  });

  it("hw_08: an unplugged scale says it didn't answer", async () => {
    renderPage();
    const plugged = await screen.findByRole("checkbox", { name: "Scale plugged in" });
    await userEvent.click(plugged);
    await userEvent.click(screen.getByRole("button", { name: /Set weight/ }));
    await userEvent.click(screen.getByRole("button", { name: /Read scale/ }));

    expect(await screen.findByRole("alert")).toHaveTextContent("The scale didn't answer in time");
    await userEvent.click(plugged);
    await userEvent.click(screen.getByRole("button", { name: /Set weight/ }));
  });

  it("hw_01/hw_10: the scan simulator types a code into the last box, then Enter", async () => {
    renderPage();
    await userEvent.click(screen.getByRole("textbox", { name: "Scan here" }));
    await userEvent.click(screen.getByRole("button", { name: /Scan simulator/ }));
    const simulator = screen.getByRole("region", { name: "Scan simulator" });

    await userEvent.click(within(simulator).getByRole("button", { name: /2112345012506/ }));

    expect(await screen.findByText(/Received 2112345012506 in \d+ ms/)).toBeVisible();
    expect(screen.getByRole("textbox", { name: "Scan here" })).toHaveValue("");
  });

  it("hw_10: the scan simulator asks for a box first when none had the cursor", async () => {
    renderPage();
    await userEvent.keyboard("{Control>}{Shift>}s{/Shift}{/Control}");
    const simulator = await screen.findByRole("region", { name: "Scan simulator" });

    await userEvent.click(within(simulator).getByRole("button", { name: /96385074/ }));

    expect(within(simulator).getByRole("alert")).toHaveTextContent("Click in a box first");
  });
});

describe("typeLikeScanner", () => {
  it("hw_01: types each character, then presses Enter", async () => {
    const input = document.createElement("input");
    document.body.append(input);
    const keys: string[] = [];
    input.addEventListener("keydown", (event) => keys.push(event.key));

    await typeLikeScanner(input, "123", 0);

    expect(input.value).toBe("123");
    expect(keys).toEqual(["1", "2", "3", "Enter"]);
    input.remove();
  });
});
