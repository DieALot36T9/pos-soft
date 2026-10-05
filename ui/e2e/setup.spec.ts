import { expect, test } from "@playwright/test";

// First run (SET-01 to SET-12), against the fake bridge.

test("set_01..set_12: a fresh install goes through the wizard into the app", async ({ page }) => {
  await page.goto("/?shop=fresh");
  await page.getByRole("button", { name: /Set up a new shop/ }).click();

  await page.getByRole("textbox", { name: "Shop name" }).fill("Rahim Store");
  await page.getByRole("button", { name: /^Next/ }).click();

  await expect(page.getByTestId("money-preview")).toHaveText("৳12,34,567.50");
  await page.getByRole("combobox", { name: "Currency" }).selectOption("USD");
  await expect(page.getByTestId("money-preview")).toHaveText("$1,234,567.50");
  await page.getByRole("button", { name: /^Next/ }).click();

  await page.getByRole("textbox", { name: "Rate name" }).fill("Standard 15%");
  await page.getByRole("textbox", { name: "Percent" }).fill("15");
  await page.getByRole("button", { name: /Add rate/ }).click();
  await expect(page.getByRole("list", { name: "Tax rates" })).toContainText("15%");
  await page.getByRole("button", { name: /^Next/ }).click();

  await page.getByRole("textbox", { name: "Username" }).fill("owner");
  await page.getByRole("textbox", { name: "Your name" }).fill("Shop Owner");
  await page.getByLabel("Password", { exact: true }).fill("shop-owner-1");
  await page.getByLabel("Password again").fill("shop-owner-1");
  await page.getByRole("button", { name: /^Next/ }).click();
  await expect(page.getByTestId("recovery-code")).toBeVisible();
  await page.getByRole("button", { name: /I have saved it/ }).click();

  await page.getByRole("textbox", { name: "Till code" }).fill("T2");
  await page.getByRole("button", { name: /^Next/ }).click();
  await page.getByRole("button", { name: /Skip for now/ }).click();
  await page.getByRole("button", { name: /^Next/ }).click();
  await page.getByRole("button", { name: /^Next/ }).click();

  await expect(page.getByRole("heading", { name: "Your shop is ready" })).toBeVisible();
  await page.getByRole("button", { name: /^Start/ }).click();
  await expect(page.getByRole("navigation", { name: "Main menu" })).toBeVisible();
  await expect(page.getByText("Signed in: Shop Owner")).toBeVisible();
});

test("set_02: Back shows what was saved", async ({ page }) => {
  await page.goto("/?shop=fresh");
  await page.getByRole("button", { name: /Set up a new shop/ }).click();
  await page.getByRole("textbox", { name: "Shop name" }).fill("Corner Shop");
  await page.getByRole("button", { name: /^Next/ }).click();
  await expect(page.getByTestId("money-preview")).toBeVisible();

  await page.getByRole("button", { name: /^Back/ }).click();

  await expect(page.getByRole("textbox", { name: "Shop name" })).toHaveValue("Corner Shop");
});
