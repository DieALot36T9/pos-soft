import { expect, test } from "@playwright/test";

// Settings, tax rates and backups after setup (SET-13, TAX-02, BAK-04, BAK-05).

test("set_13: the admin logs in and changes the shop name", async ({ page }) => {
  await page.goto("/?shop=signed_out");
  await page.getByRole("button", { name: /Shop Owner/ }).click();
  await page.getByLabel("Password").fill("shop-owner-1");
  await page.getByRole("button", { name: /^Log in/ }).click();

  await page
    .getByRole("navigation", { name: "Main menu" })
    .getByRole("button", { name: /Settings/ })
    .click();
  const name = page.getByRole("textbox", { name: "Shop name" });
  await expect(name).toHaveValue("Rahim Store");
  await name.fill("Rahim Super Store");
  await page.getByRole("button", { name: /^Save/ }).click();
  await expect(page.getByText("Saved.")).toBeVisible();
});

test("tax_02: adds a tax rate", async ({ page }) => {
  await page.goto("/");
  await page
    .getByRole("navigation", { name: "Main menu" })
    .getByRole("button", { name: /Tax rates/ })
    .click();
  await page.getByRole("textbox", { name: "Name" }).fill("Reduced 5%");
  await page.getByRole("textbox", { name: "Percent" }).fill("5");
  await page.getByRole("button", { name: /Save rate/ }).click();
  await expect(page.getByRole("cell", { name: "Reduced 5%" })).toBeVisible();
  await expect(page.getByRole("cell", { name: "5%", exact: true })).toBeVisible();
});

test("bak_04/bak_05: backs up now, then asks before a restore", async ({ page }) => {
  await page.goto("/");
  await page
    .getByRole("navigation", { name: "Main menu" })
    .getByRole("button", { name: /Backups/ })
    .click();
  await page.getByRole("button", { name: /Back up now/ }).click();
  await expect(page.getByText(/Backed up to/)).toBeVisible();
  await expect(page.getByRole("cell", { name: "By hand" })).toBeVisible();

  await page
    .getByRole("button", { name: /Put this backup back/ })
    .first()
    .click();
  await expect(page.getByRole("alertdialog")).toContainText("Restore the backup from");
  await page.getByRole("button", { name: /^Cancel/ }).click();
  await expect(page.getByRole("alertdialog")).toHaveCount(0);
});
