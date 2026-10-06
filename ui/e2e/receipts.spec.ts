import { expect, test, type Page } from "@playwright/test";

// Receipts and devices at the till (milestone M7): the receipt shows on the Preview
// printer with the drawer pulse, Ctrl+P reprints it as a COPY, the X report prints
// from the shift menu, and an offline printer shows the banner without losing the sale.

const menu = (page: Page) => page.getByRole("navigation", { name: "Main menu" });

async function openTill(page: Page) {
  await menu(page)
    .getByRole("button", { name: /Checkout/ })
    .click();
  await expect(page.getByText("Open a shift to start selling")).toBeVisible();
  await page.getByRole("button", { name: /Open shift/ }).click();
  await page.getByRole("textbox", { name: "Opening float" }).fill("1000");
  await page.keyboard.press("Enter");
  await expect(page.getByText("Scan an item to start the sale.")).toBeVisible();
}

async function sellMilkForCash(page: Page) {
  await page.keyboard.type("8901234567890");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("row").filter({ hasText: "Milk 1L" })).toBeVisible();
  await page.keyboard.press("F10");
  await page
    .getByRole("dialog", { name: "Payment" })
    .getByRole("textbox", { name: "Cash" })
    .fill("100");
  await page.keyboard.press("Enter");
  await expect(page.getByText("Sale complete · Invoice T1-000001")).toBeVisible();
}

test("hw_04/pay_10/shf_05: the receipt prints with the drawer, Ctrl+P reprints, F11 prints the X report", async ({
  page,
}) => {
  await page.goto("/");
  await openTill(page);
  await sellMilkForCash(page);

  const preview = page.getByRole("complementary", { name: "Preview printer" });
  await expect(preview.getByRole("img", { name: "The newest printout" })).toBeVisible();
  await expect(preview).toContainText("Drawer opened");

  await page.keyboard.press("Control+p");
  await expect(page.getByText("The last receipt was printed again, marked COPY.")).toBeVisible();

  await page.keyboard.press("F11");
  await page.getByRole("button", { name: /X report/ }).click();
  await expect(page.getByText("X report printed.")).toBeVisible();
});

test("pay_09: an offline printer keeps the sale and offers Retry and the system printer", async ({
  page,
}) => {
  await page.goto("/");
  await menu(page)
    .getByRole("button", { name: /Settings/ })
    .click();
  await page.getByRole("button", { name: "Printer and scale" }).click();
  await page.getByRole("combobox", { name: "Receipt printer" }).selectOption("tcp");
  await page.getByRole("textbox", { name: "IP address" }).fill("10.0.0.9");
  await page.getByRole("button", { name: /^Save/ }).click();
  await expect(page.getByText("Saved.")).toBeVisible();

  await openTill(page);
  await sellMilkForCash(page);
  await expect(page.getByText("Receipt not printed. The sale is saved.")).toBeVisible();
  await page.getByRole("button", { name: /Retry/ }).click();
  await expect(page.getByText("Receipt not printed. The sale is saved.")).toBeVisible();
  await page.getByRole("button", { name: /Print to system printer/ }).click();
  await page
    .getByRole("button", { name: /^Close/ })
    .first()
    .click();
  await expect(page.getByText("Receipt not printed. The sale is saved.")).toBeHidden();
  // The next customer can be served as usual.
  await expect(page.getByText("Scan an item to start the sale.")).toBeVisible();
});
