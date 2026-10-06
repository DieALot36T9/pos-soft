import { expect, test, type Page } from "@playwright/test";

// The fake shop starts set up with the admin signed in; the page is in the main menu.
async function openHardwarePage(page: Page) {
  await page.goto("/");
  await page
    .getByRole("navigation", { name: "Main menu" })
    .getByRole("button", { name: /Printer and devices/ })
    .click();
}

// Working without hardware (rule HW-10), against the fake bridge.

test("hw_02/hw_10: a test print at 58 mm shows the receipt picture in the Preview printer", async ({
  page,
}) => {
  await openHardwarePage(page);
  const preview = page.getByRole("region", { name: "Preview printer" });
  await expect(preview.getByText(/Nothing printed yet/)).toBeVisible();

  await page.getByRole("radio", { name: "58 mm" }).check();
  await page.getByRole("button", { name: /Test print/ }).click();

  const picture = preview.getByRole("img", { name: "Printout 1" });
  await expect(picture).toBeVisible();
  // The picture is shown dot for dot: 58 mm paper is 384 dots wide.
  await expect.poll(() => picture.evaluate((img: HTMLImageElement) => img.naturalWidth)).toBe(384);
  await expect(preview.getByRole("button", { name: /Print on another printer/ })).toBeVisible();
});

test("hw_10: the fake scale reads a moving weight, then a still one", async ({ page }) => {
  await openHardwarePage(page);
  const weight = page.getByRole("textbox", { name: "Weight" });
  await weight.fill("2.5");
  await page.getByRole("checkbox", { name: "Weight is still" }).uncheck();
  await page.getByRole("button", { name: /Set weight/ }).click();
  await page.getByRole("button", { name: /Read scale/ }).click();
  await expect(page.getByText("Moving", { exact: true })).toBeVisible();

  await page.getByRole("checkbox", { name: "Weight is still" }).check();
  await page.getByRole("button", { name: /Set weight/ }).click();
  await page.getByRole("button", { name: /Read scale/ }).click();
  await expect(page.getByText("Still", { exact: true })).toBeVisible();
});

test("hw_01/hw_10: Ctrl+Shift+S opens the scan simulator, which scans into the box", async ({
  page,
}) => {
  await openHardwarePage(page);
  const scanBox = page.getByRole("textbox", { name: "Scan here" });
  await scanBox.click();
  await page.keyboard.press("Control+Shift+S");
  const simulator = page.getByRole("region", { name: "Scan simulator" });
  await expect(simulator).toBeVisible();

  await simulator.getByRole("button", { name: /5901234123457/ }).click();

  await expect(page.getByText(/Received 5901234123457 in \d+ ms/)).toBeVisible();
  await expect(scanBox).toBeFocused();
});

test("design: every button and field has a tooltip, shown when reached with Tab", async ({
  page,
}) => {
  await openHardwarePage(page);
  await page.getByRole("button", { name: /Scan simulator/ }).click();
  await page.getByRole("heading", { name: "Supershop POS" }).click();
  await page.keyboard.press("Tab");
  // The first control on screen shows its tooltip at once.
  await expect(page.getByRole("button", { name: /^Home/ })).toBeFocused();
  await expect(page.getByRole("tooltip")).toHaveText(
    "The start page with stock alerts and the app's version.",
  );

  // Walk the page with the Tab key. Every control sits inside a tooltip trigger
  // (Radix marks each trigger with data-state). A tooltip that opens while the page
  // scrolls the control into view closes again, so the trigger is what is checked.
  let checked = 0;
  for (let step = 0; step < 40; step += 1) {
    const control = await page.evaluate(() => {
      const element = document.activeElement;
      if (!element || !["BUTTON", "INPUT", "SELECT"].includes(element.tagName)) {
        return null;
      }
      return {
        name: element.getAttribute("aria-label") ?? element.id ?? element.tagName,
        hasTip: element.closest("[data-state]") !== null,
      };
    });
    if (control) {
      expect(control.hasTip, `tooltip for ${control.name}`).toBe(true);
      checked += 1;
    }
    await page.keyboard.press("Tab");
  }
  expect(checked).toBeGreaterThan(15);
});
