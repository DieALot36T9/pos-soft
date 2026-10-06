import { expect, test, type Page } from "@playwright/test";

// Returns, exchanges and voids at the till (milestone M9): a receipt is found by its
// invoice number, part of it comes back for cash with a return receipt, an exchange
// puts the refund on the next sale, and a sale from this shift is voided.

const menu = (page: Page) => page.getByRole("navigation", { name: "Main menu" });

async function openTill(page: Page) {
  await menu(page)
    .getByRole("button", { name: /Checkout/ })
    .click();
  await page.getByRole("button", { name: /Open shift/ }).click();
  await page.getByRole("textbox", { name: "Opening float" }).fill("1000");
  await page.keyboard.press("Enter");
  await expect(page.getByText("Scan an item to start the sale.")).toBeVisible();
}

// Sells 2 × Milk 1L (90.00 each) for cash: invoice T1-000001.
async function sellTwoMilk(page: Page) {
  await page.keyboard.type("2*8901234567890");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("row").filter({ hasText: "Milk 1L" })).toBeVisible();
  await page.keyboard.press("F10");
  await page
    .getByRole("dialog", { name: "Payment" })
    .getByRole("textbox", { name: "Cash" })
    .fill("200");
  await page.keyboard.press("Enter");
  await expect(page.getByText("Sale complete · Invoice T1-000001")).toBeVisible();
}

async function findReceipt(page: Page) {
  await page.keyboard.press("Control+r");
  const dialog = page.getByRole("dialog", { name: "Return, exchange or void" });
  await dialog.getByRole("textbox", { name: "Invoice number" }).fill("T1-000001");
  await page.keyboard.press("Enter");
  await expect(dialog.getByText(/Invoice T1-000001/)).toBeVisible();
  return dialog;
}

test("ret_01/ret_06: one of two comes back for cash and the return receipt prints", async ({
  page,
}) => {
  await page.goto("/");
  await openTill(page);
  await sellTwoMilk(page);
  const dialog = await findReceipt(page);
  await dialog.getByRole("textbox", { name: "Coming back from Milk 1L" }).fill("1");
  await dialog.getByRole("textbox", { name: "Reason" }).fill("Wrong size");
  // Rust (here the fake bridge) works out the refund before anything is saved.
  await expect(dialog.getByText("৳90.00").first()).toBeVisible();
  await dialog.getByRole("button", { name: /^Refund/ }).click();
  await expect(page.getByText("Return T1-R000001 saved")).toBeVisible();
  await expect(page.getByRole("status").getByText("৳90.00")).toBeVisible();
  const preview = page.getByRole("complementary", { name: "Preview printer" });
  await expect(preview).toContainText("Drawer opened");

  // Only one is left to return now.
  const again = await findReceipt(page);
  await again.getByRole("textbox", { name: "Coming back from Milk 1L" }).fill("2");
  await expect(again.getByText("Only 1 can still come back on this line.")).toBeVisible();
});

test("ret_05: an exchange puts the refund on the next sale", async ({ page }) => {
  await page.goto("/");
  await openTill(page);
  await sellTwoMilk(page);
  const dialog = await findReceipt(page);
  await dialog.getByRole("textbox", { name: "Coming back from Milk 1L" }).fill("1");
  await dialog.getByRole("textbox", { name: "Reason" }).fill("Wants another");
  await dialog.getByRole("button", { name: /^Exchange/ }).click();
  await expect(page.getByText("Exchange from T1-R000001: ৳90.00 to spend")).toBeVisible();
  await page.keyboard.type("8901234567890");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("row").filter({ hasText: "Milk 1L" })).toBeVisible();
  await page.keyboard.press("F10");
  const payment = page.getByRole("dialog", { name: "Payment" });
  await expect(payment.getByText("Exchange credit")).toBeVisible();
  await page.keyboard.press("Enter");
  await expect(page.getByText("Sale complete · Invoice T1-000002")).toBeVisible();
  await expect(page.getByText(/Exchange from/)).toBeHidden();
});

test("ret_04: a sale from this shift is voided with a reason", async ({ page }) => {
  await page.goto("/");
  await openTill(page);
  await sellTwoMilk(page);
  const dialog = await findReceipt(page);
  await dialog.getByRole("button", { name: /^Void sale/ }).click();
  const confirm = page.getByRole("dialog", { name: "Void sale T1-000001?" });
  await confirm.getByRole("textbox", { name: "Reason" }).fill("Rang up twice");
  await confirm.getByRole("button", { name: /^Void sale/ }).click();
  await expect(page.getByText("Sale T1-000001 voided")).toBeVisible();
  await expect(page.getByRole("status").getByText("৳180.00")).toBeVisible();
});
