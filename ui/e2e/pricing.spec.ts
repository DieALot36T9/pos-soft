import { expect, test, type Page } from "@playwright/test";

// Prices, promotions and customers (milestone C-M8), in a real browser against the fake
// bridge: the till by keyboard, then the pricing and customer screens in the back office.

async function openTill(page: Page) {
  await page.goto("/");
  await page
    .getByRole("navigation", { name: "Main menu" })
    .getByRole("button", { name: /Checkout/ })
    .click();
  await page.getByRole("button", { name: /Open shift/ }).click();
  await page.getByRole("textbox", { name: "Opening float" }).fill("1000");
  await page.keyboard.press("Enter");
  await expect(page.getByText("Scan an item to start the sale.")).toBeVisible();
}

async function openOffice(page: Page, menu: RegExp) {
  await page.goto("/");
  await page
    .getByRole("navigation", { name: "Main menu" })
    .getByRole("button", { name: menu })
    .click();
}

test("chk_11/chk_12/chk_13: customer, discount and a typed price at the till", async ({ page }) => {
  await openTill(page);
  await page.keyboard.type("8901234567890");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("row").filter({ hasText: "Milk 1L" })).toBeVisible();

  await page.keyboard.press("F3");
  const customer = page.getByRole("dialog", { name: "Customer" });
  await customer.getByRole("textbox", { name: "Phone or name" }).fill("Asha");
  await expect(customer.getByRole("option")).toHaveCount(1);
  await page.keyboard.press("Enter");
  await expect(page.getByText("Asha Begum · +8801711123456")).toBeVisible();
  await expect(page.getByText("Gold price")).toBeVisible();

  await page.keyboard.press("F7");
  const discount = page.getByRole("dialog", { name: "Discount on Milk 1L" });
  await discount.getByRole("textbox", { name: "Percent off" }).fill("10");
  await page.keyboard.press("Enter");
  // 90.00 − 5% Gold (4.50) − 10% of the rest (8.55) = 76.95, rounded for cash to 77.00.
  await expect(page.getByLabel("TOTAL")).toContainText("৳77.00");

  await page.keyboard.press("F8");
  const price = page.getByRole("dialog", { name: "Price for Milk 1L" });
  await price.getByRole("textbox", { name: "New price for one" }).fill("100");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("row").filter({ hasText: "Milk 1L" })).toContainText("Price typed");

  await page.keyboard.press("F10");
  await page.keyboard.press("Enter");
  await expect(page.getByText(/Sale complete · Invoice T1-000001/)).toBeVisible();
  await expect(page.getByText("No customer")).toBeVisible();
});

test("prc_02/prc_04: a price list with a price, and a promotion", async ({ page }) => {
  await openOffice(page, /Prices and promotions/);
  await expect(page.getByRole("heading", { name: "Prices and promotions" })).toBeVisible();
  const wholesale = page.getByRole("row").filter({ hasText: "Wholesale" });
  await wholesale.getByRole("button", { name: "See and change the prices in this list." }).click();
  const prices = page.getByRole("region", { name: "Prices in Wholesale" });
  await prices.getByRole("textbox", { name: "Find a product" }).fill("milk");
  await prices.getByRole("button", { name: "Milk 1L" }).click();
  await prices.getByRole("textbox", { name: "Price in this list" }).fill("80");
  await prices.getByRole("button", { name: /Set price/ }).click();
  await expect(prices.getByRole("row").filter({ hasText: "Milk 1L" })).toContainText("৳80.00");

  await page.getByRole("tab", { name: "Promotions" }).click();
  await page.getByRole("textbox", { name: "Name" }).fill("Soap week");
  await page.getByRole("textbox", { name: "Percent off" }).fill("10");
  await page.getByRole("combobox", { name: "Category" }).selectOption({ index: 1 });
  await page.getByRole("button", { name: /Add category/ }).click();
  await page.getByRole("button", { name: /Save/ }).click();
  await expect(page.getByRole("row").filter({ hasText: "Soap week" })).toContainText("10% off");
});

test("cus_01/cus_04: add a customer and correct their points", async ({ page }) => {
  await openOffice(page, /Customers/);
  await expect(page.getByRole("heading", { name: "Customers" })).toBeVisible();
  await page.getByRole("textbox", { name: "Name", exact: true }).fill("Karim");
  await page.getByRole("textbox", { name: "Mobile", exact: true }).fill("01912 345678");
  await page.getByRole("button", { name: /Add customer/ }).click();
  await page.getByRole("button", { name: "Karim" }).click();
  await expect(page.getByRole("heading", { name: "Karim" })).toBeVisible();

  await page.getByRole("textbox", { name: "Points", exact: true }).fill("25");
  await page.getByRole("textbox", { name: "Reason", exact: true }).fill("Welcome gift");
  await page.getByRole("button", { name: /Correct points/ }).click();
  await expect(page.getByText("Points now: 25.")).toBeVisible();
});
