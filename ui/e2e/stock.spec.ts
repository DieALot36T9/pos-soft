import { expect, test, type Page } from "@playwright/test";

// Stock levels, adjustments, history, the dashboard alerts and stock counts (INV-01 to INV-10).

function menu(page: Page) {
  return page.getByRole("navigation", { name: "Main menu" });
}

async function openMenu(page: Page, entry: RegExp, heading: string) {
  await page.goto("/");
  await menu(page).getByRole("button", { name: entry }).click();
  await expect(page.getByRole("heading", { name: heading, exact: true })).toBeVisible();
}

test("inv_03/inv_05: the dashboard shows low stock and batches near expiry", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Stock alerts" })).toBeVisible();
  await expect(page.getByText("Milk 1L: 24 pc (reorder below 30)")).toBeVisible();
  await expect(page.getByText("Milk 1L: 10 pc, expires 2026-10-12")).toBeVisible();
  await page.getByRole("button", { name: /Open stock/ }).click();
  await expect(page.getByRole("heading", { name: "Stock", exact: true })).toBeVisible();
});

test("inv_02/inv_07: adjust stock with a reason and see it in the history", async ({ page }) => {
  await openMenu(page, /^Stock$/, "Stock");
  const bread = page.getByRole("row").filter({ hasText: "Bread" });
  await bread.getByRole("button", { name: /Open the stock/ }).click();
  await expect(page.getByRole("heading", { name: "Bread" })).toBeVisible();

  await page.getByRole("button", { name: /Adjust stock/ }).click();
  await page.getByRole("combobox", { name: "Way" }).selectOption("out");
  await page.getByRole("textbox", { name: "Quantity" }).fill("3");
  await page.getByRole("combobox", { name: "Reason" }).selectOption("theft");
  await page.getByRole("textbox", { name: "Note" }).fill("seen on camera");
  await page.getByRole("button", { name: /^Save/ }).click();

  await expect(page.getByText("Stock saved. On hand now: 9 pc.")).toBeVisible();
  const row = page.getByRole("row").filter({ hasText: "Theft: seen on camera" });
  await expect(row.getByRole("cell", { name: "-3", exact: true })).toBeVisible();
  await expect(row.getByRole("cell", { name: "9", exact: true })).toBeVisible();

  // Esc goes back to the list, which shows the new stock.
  await page.keyboard.press("Escape");
  await expect(page.getByRole("row").filter({ hasText: "Bread" }).getByText("9 pc")).toBeVisible();
});

test("inv_06: a full count by scanning, reviewed and posted", async ({ page }) => {
  await openMenu(page, /Stock counts/, "Stock counts");
  await page.getByRole("textbox", { name: "Name" }).fill("Month end");
  await page.getByRole("button", { name: /Start count/ }).click();
  await expect(page.getByRole("heading", { name: /SC-000001: Month end/ })).toBeVisible();

  const code = page.getByRole("textbox", { name: /Scan or type/ });
  const qty = page.getByRole("textbox", { name: "Quantity" });
  await code.fill("8902222222227");
  await qty.fill("39");
  await qty.press("Enter");
  await expect(page.getByText("Counted 39 pc of Soap.")).toBeVisible();
  // Milk has batches: pick one, then Add.
  await code.fill("8901234567890");
  await code.press("Enter");
  await page
    .getByRole("combobox", { name: "Batch" })
    .selectOption({ label: "Batch M-0921, 2026-10-12" });
  await page.getByRole("button", { name: /^Add/ }).click();
  await expect(page.getByText("Counted 1 pc of Milk 1L.")).toBeVisible();

  await page.getByRole("button", { name: /Review differences/ }).click();
  await expect(page.getByRole("heading", { name: /Differences for SC-000001/ })).toBeVisible();
  await expect(page.getByText(/products with stock were not counted/)).toBeVisible();
  await page.getByRole("checkbox", { name: /Set uncounted products to 0/ }).check();
  await page.getByRole("button", { name: /Post count/ }).click();
  await expect(page.getByText(/Posted: \d+ stock changes/)).toBeVisible();

  await page.getByRole("main").getByRole("button", { name: /^Back/ }).click();
  await expect(page.getByText("This count is closed. It can't be changed.")).toBeVisible();
});
