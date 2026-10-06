import { expect, test, type Page } from "@playwright/test";

// Products, categories, brands and units (CAT-01 to CAT-11), mostly by keyboard.

function menu(page: Page) {
  return page.getByRole("navigation", { name: "Main menu" });
}

async function openProducts(page: Page) {
  await page.goto("/");
  await menu(page)
    .getByRole("button", { name: /Products/ })
    .click();
  await expect(page.getByRole("heading", { name: "Products" })).toBeVisible();
}

test("cat_01/cat_09: add a product and change its price by keyboard", async ({ page }) => {
  await openProducts(page);
  await page.getByRole("button", { name: /Add product/ }).click();

  await page.getByRole("textbox", { name: "Name" }).fill("Rice 5kg");
  await page.getByRole("tab", { name: /Prices and tax/ }).press("Enter");
  await page.getByRole("textbox", { name: "Sale price" }).fill("650");
  await page.getByRole("tab", { name: /Barcodes/ }).press("Enter");
  await page.getByRole("textbox", { name: "New barcode" }).fill("8901234567906");
  await page.getByRole("textbox", { name: "New barcode" }).press("Enter");
  await expect(page.getByText("8901234567906")).toBeVisible();
  await page.keyboard.press("Control+s");
  await expect(page.getByRole("heading", { name: "Change Rice 5kg" })).toBeVisible();

  await page.getByRole("tab", { name: /Prices and tax/ }).press("Enter");
  await page.getByRole("textbox", { name: "Sale price" }).fill("700");
  await page.keyboard.press("Control+s");
  await expect(page.getByRole("cell", { name: "৳700.00", exact: true })).toBeVisible();
  await expect(page.getByRole("cell", { name: "Changed by hand", exact: true })).toBeVisible();

  await page.keyboard.press("Escape");
  await expect(page.getByRole("cell", { name: "Rice 5kg", exact: true })).toBeVisible();
  await expect(page.getByRole("cell", { name: "৳700.00", exact: true })).toBeVisible();
});

test("cat_04/cat_11: duplicate barcodes are refused and a shop barcode is made", async ({
  page,
}) => {
  await openProducts(page);
  await page.getByRole("button", { name: /Add product/ }).click();
  await page.getByRole("textbox", { name: "Name" }).fill("Milk copy");
  await page.getByRole("tab", { name: /Barcodes/ }).click();
  await page.getByRole("textbox", { name: "New barcode" }).fill("8901234567890");
  await page.getByRole("textbox", { name: "New barcode" }).press("Enter");
  await page.getByRole("button", { name: /Save product/ }).click();
  await expect(
    page.getByText("Barcode 8901234567890 is already used by Milk 1L (SKU P000001).").first(),
  ).toBeVisible();

  await page.getByRole("button", { name: /Take this barcode off/ }).click();
  await page.getByRole("button", { name: /Save product/ }).click();
  await expect(page.getByRole("heading", { name: "Change Milk copy" })).toBeVisible();
  await page.getByRole("button", { name: /Make shop barcode/ }).click();
  await expect(page.getByText("Barcode 2000000000015 was added.")).toBeVisible();
});

test("cat_06: a parent product makes its variants", async ({ page }) => {
  await openProducts(page);
  await page.getByRole("button", { name: /Add product/ }).click();
  await page.getByRole("textbox", { name: "Name" }).fill("Shampoo");
  await page.getByRole("combobox", { name: "Kind" }).selectOption("variant_parent");
  await page.getByRole("button", { name: /Save product/ }).click();
  await page.getByRole("tab", { name: /Variants/ }).click();
  await page.getByRole("textbox", { name: "Attribute" }).fill("Size");
  await page.getByRole("textbox", { name: "Values" }).fill("200 ml, 400 ml");
  await page.getByRole("button", { name: /Make variants/ }).click();
  await expect(page.getByText("2 new variants were made.")).toBeVisible();
  await expect(page.getByRole("cell", { name: "Shampoo 400 ml", exact: true })).toBeVisible();
});

test("cat_08: archive and bring back", async ({ page }) => {
  await openProducts(page);
  const bread = page.getByRole("row", { name: /Bread/ });
  await bread.getByRole("button", { name: /Open this product/ }).click();
  await page.getByRole("button", { name: /^Archive/ }).click();
  await expect(page.getByText("The product was archived.")).toBeVisible();
  await page.getByRole("button", { name: /Back to list/ }).click();
  await expect(page.getByRole("cell", { name: "Bread", exact: true })).toHaveCount(0);
  await page.getByLabel("Show archived products").check();
  await expect(page.getByRole("row", { name: /Bread/ }).getByText("Archived")).toBeVisible();
});

test("cat_02/cat_03: units, categories and brands", async ({ page }) => {
  await page.goto("/");
  await menu(page).getByRole("button", { name: /Units/ }).click();
  await page.getByRole("textbox", { name: "Name" }).fill("Dozen");
  await page.getByRole("textbox", { name: "Sign" }).fill("dz");
  await page.getByRole("button", { name: /Save unit/ }).click();
  await expect(page.getByRole("cell", { name: "Dozen", exact: true })).toBeVisible();

  await menu(page)
    .getByRole("button", { name: /Categories/ })
    .click();
  await page.getByRole("textbox", { name: "Name" }).fill("Milk");
  await page.getByRole("combobox", { name: "Inside" }).selectOption({ label: "Dairy" });
  await page.getByRole("button", { name: /Save category/ }).click();
  await expect(page.getByRole("cell", { name: "Dairy › Milk", exact: true })).toBeVisible();

  await menu(page)
    .getByRole("button", { name: /Brands/ })
    .click();
  await page.getByRole("textbox", { name: "Name" }).fill("Aarong");
  await page.getByRole("textbox", { name: "Name" }).press("Enter");
  await expect(page.getByRole("cell", { name: "Aarong", exact: true })).toBeVisible();
});
