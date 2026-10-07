import { expect, test, type Page } from "@playwright/test";

// Reports in the back office (milestone M11): a report is picked, run for a range of
// days, printed on receipt paper and A4, and saved as Excel. Every number comes from
// the fake bridge, as from Rust.

const menu = (page: Page) => page.getByRole("navigation", { name: "Main menu" });

async function openReports(page: Page) {
  await page.goto("/");
  await menu(page)
    .getByRole("button", { name: /Reports/ })
    .click();
  await expect(page.getByRole("heading", { name: "Reports", exact: true })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "Business day" })).toHaveValue("2026-03-02");
}

test("rpt_01/rpt_09: the daily summary is shown, printed and saved as Excel", async ({ page }) => {
  await openReports(page);
  await page.getByRole("button", { name: /^Show/ }).click();
  const report = page.getByRole("article", { name: "Daily summary" });
  await expect(report).toContainText("Rahim Store");
  await expect(report.getByRole("cell", { name: "৳425.00" }).first()).toBeVisible();

  await page.getByRole("button", { name: /^Print receipt/ }).click();
  await expect(page.getByText("The report was printed.")).toBeVisible();
  await page.getByRole("button", { name: /^Excel/ }).click();
  await expect(page.getByText("The report was saved as an Excel file.")).toBeVisible();
  await page.getByRole("button", { name: /^Print A4 or PDF/ }).click();
  await expect(page.getByRole("alert")).toHaveCount(0);
});

test("rpt_03: sales analysis for this month, with a backwards range refused", async ({ page }) => {
  await openReports(page);
  await page.getByRole("button", { name: /^Sales analysis/ }).click();
  await page.getByRole("combobox", { name: "Days" }).selectOption("this_month");
  await expect(page.getByRole("textbox", { name: "From" })).toHaveValue("2026-03-01");
  await page.getByRole("combobox", { name: "Group by" }).selectOption("category");
  await page.getByRole("button", { name: /^Show/ }).click();
  await expect(page.getByText("Period: 2026-03-01 to 2026-03-02")).toBeVisible();
  await expect(page.getByRole("cell", { name: "60%" })).toBeVisible();

  await page.getByRole("textbox", { name: "From" }).fill("2026-03-05");
  await page.getByRole("button", { name: /^Show/ }).click();
  await expect(page.getByText("The first day must be on or before the last day.")).toBeVisible();
});

test("rpt_07: a customer statement needs a customer found first", async ({ page }) => {
  await openReports(page);
  await page.getByRole("button", { name: /^Customer statement/ }).click();
  await page.getByRole("button", { name: /^Show/ }).click();
  await expect(page.getByText("Find and choose a customer first.")).toBeVisible();
  await page.getByRole("textbox", { name: "Find customer" }).fill("asha");
  await page.getByRole("button", { name: /^Find/ }).click();
  await expect(page.getByRole("combobox", { name: "Customer" })).toContainText("Asha");
  await page.getByRole("button", { name: /^Show/ }).click();
  await expect(page.getByRole("article")).toBeVisible();
});
