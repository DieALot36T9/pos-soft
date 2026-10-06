import { expect, test, type Page } from "@playwright/test";

// A cashier's day at the till by keyboard only (milestone M6): open a shift, scan,
// change a quantity, weigh, hold and resume, pay with change, and close the shift.

async function openTill(page: Page) {
  await page.goto("/");
  await page
    .getByRole("navigation", { name: "Main menu" })
    .getByRole("button", { name: /Checkout/ })
    .click();
  await expect(page.getByText("Open a shift to start selling")).toBeVisible();
  await page.getByRole("button", { name: /Open shift/ }).click();
  await page.getByRole("textbox", { name: "Opening float" }).fill("1000");
  await page.keyboard.press("Enter");
  await expect(page.getByText("Scan an item to start the sale.")).toBeVisible();
}

const scanBox = (page: Page) => page.getByRole("textbox", { name: "Scan or search" });

test("chk_01/pay_01: scan, change the quantity and pay in cash with change", async ({ page }) => {
  await openTill(page);
  await expect(scanBox(page)).toBeFocused();
  await page.keyboard.type("8901234567890");
  await page.keyboard.press("Enter");
  const milk = page.getByRole("row").filter({ hasText: "Milk 1L" });
  await expect(milk).toBeVisible();
  await page.keyboard.press("F4");
  await page.getByRole("textbox", { name: "Quantity" }).fill("3");
  await page.keyboard.press("Enter");
  await expect(page.getByLabel("TOTAL")).toContainText("৳270.00");

  await page.keyboard.press("F10");
  const payment = page.getByRole("dialog", { name: "Payment" });
  await expect(payment.getByRole("textbox", { name: "Cash" })).toHaveValue("270.00");
  await payment.getByRole("textbox", { name: "Cash" }).fill("500");
  await expect(payment.getByLabel("CHANGE")).toContainText("৳230.00");
  await page.keyboard.press("Enter");
  await expect(page.getByText("Sale complete · Invoice T1-000001")).toBeVisible();
  await expect(page.getByText("৳230.00")).toBeVisible();
  await expect(scanBox(page)).toBeFocused();
});

test("chk_06/chk_09/chk_10: weigh an item, hold the sale and bring it back", async ({ page }) => {
  await openTill(page);
  await page.keyboard.type("2000000000015");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog", { name: "Weigh Potato" })).toBeVisible();
  await page.keyboard.press("m");
  await page.getByRole("textbox", { name: "Weight" }).fill("1.25");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("row").filter({ hasText: "Potato" })).toContainText("1.250");

  await page.keyboard.press("F5");
  await page.getByRole("textbox", { name: "Note (optional)" }).fill("comes back later");
  await page.keyboard.press("Enter");
  await expect(page.getByText("Scan an item to start the sale.")).toBeVisible();
  await page.keyboard.press("F6");
  await expect(page.getByRole("dialog", { name: "Sales on hold" })).toContainText(
    "comes back later",
  );
  await page.keyboard.press("Enter");
  await expect(page.getByRole("row").filter({ hasText: "Potato" })).toBeVisible();
});

test("shf_06: close the shift with the count", async ({ page }) => {
  await openTill(page);
  await page.keyboard.press("F11");
  await page.getByRole("button", { name: /Close shift/ }).click();
  await page.getByRole("textbox", { name: "৳1,000.00" }).fill("1");
  await page.getByRole("button", { name: /Close shift/ }).click();
  const summary = page.getByRole("dialog", { name: "Shift closed" });
  await expect(summary).toContainText("Exact");
  await page.keyboard.press("Enter");
  await expect(page.getByText("Open a shift to start selling")).toBeVisible();
});
