import { expect, test, type Page } from "@playwright/test";

// Purchasing in the back office (milestone M10): an order for a supplier is made and
// exported, the goods are received against it, the supplier is paid, and goods are
// sent back. Every amount comes from the fake bridge, as from Rust.

const menu = (page: Page) => page.getByRole("navigation", { name: "Main menu" });

async function openPurchasing(page: Page) {
  await page.goto("/");
  await menu(page)
    .getByRole("button", { name: /Purchasing/ })
    .click();
  await expect(page.getByRole("heading", { name: "Purchasing" })).toBeVisible();
}

async function pickMilk(page: Page) {
  await page.getByRole("textbox", { name: "Add a product" }).fill("milk");
  await page.getByRole("button", { name: "Milk 1L" }).click();
}

test("pur_02/pur_03: an order is made, exported and received in full", async ({ page }) => {
  await openPurchasing(page);
  await page.getByRole("tab", { name: "Purchase orders" }).click();
  await page.getByRole("button", { name: /^New order/ }).click();
  await pickMilk(page);
  await page.getByRole("textbox", { name: "Quantity of Milk 1L" }).fill("24");
  await page.getByRole("textbox", { name: "Cost of Milk 1L" }).fill("75");
  await page.getByRole("button", { name: /^Save order/ }).click();
  await expect(page.getByText("Status: Draft")).toBeVisible();
  await expect(page.getByText("Total ৳1,800.00")).toBeVisible();
  await page.getByRole("button", { name: /^Export to Excel/ }).click();
  await expect(page.getByText("Status: Sent")).toBeVisible();

  await page.getByRole("button", { name: /^Receive goods/ }).click();
  await expect(page.getByRole("textbox", { name: "Received of Milk 1L" })).toHaveValue("24");
  await page.getByRole("textbox", { name: "Supplier invoice number" }).fill("INV-5521");
  await page.getByRole("textbox", { name: "Invoice total" }).fill("1800");
  await expect(page.getByText("Lines total ৳1,800.00")).toBeVisible();
  await page.getByRole("button", { name: /^Save delivery/ }).click();
  await expect(page.getByText(/Delivery GRN-000001 saved\./)).toBeVisible();

  await page.getByRole("tab", { name: "Purchase orders" }).click();
  await expect(page.getByRole("row").filter({ hasText: "PO-000001" })).toContainText("Received");
});

test("pur_05/pur_04: the supplier is paid and goods are sent back", async ({ page }) => {
  await openPurchasing(page);
  await page.getByRole("tab", { name: "Receive goods" }).click();
  await pickMilk(page);
  await page.getByRole("textbox", { name: "Received of Milk 1L" }).fill("10");
  await page.getByRole("textbox", { name: "Cost of Milk 1L" }).fill("75");
  await page.getByRole("textbox", { name: "Invoice total" }).fill("750");
  await page.getByRole("button", { name: /^Save delivery/ }).click();
  await expect(page.getByText(/Delivery GRN-000001 saved\./)).toBeVisible();

  await page.getByRole("tab", { name: "Suppliers" }).click();
  await page.getByRole("button", { name: "Rahim Traders" }).click();
  await expect(page.getByText("We owe ৳750.00")).toBeVisible();
  await page.getByRole("textbox", { name: "Amount" }).fill("500");
  await page.getByRole("button", { name: /^Record payment/ }).click();
  await expect(page.getByText("Payment saved. We now owe ৳250.00.")).toBeVisible();

  await page.getByRole("tab", { name: "Supplier returns" }).click();
  await page.getByRole("combobox", { name: "From delivery" }).selectOption({ label: "GRN-000001" });
  await page.getByRole("textbox", { name: "Sending back of Milk 1L" }).fill("2");
  await page.getByRole("textbox", { name: "Reason" }).fill("Leaking");
  await page.getByRole("button", { name: /^Send back/ }).click();
  await expect(page.getByText("Return SR-000001 saved: ৳150.00 off what we owe.")).toBeVisible();

  await page.getByRole("tab", { name: "What we owe" }).click();
  await expect(page.getByRole("row").filter({ hasText: "Rahim Traders" })).toContainText("৳100.00");
});
