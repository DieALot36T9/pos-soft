import { expect, test, type Page } from "@playwright/test";

// Import / Export and labels in the back office (milestone M12): a file is checked,
// its problems listed and the good rows imported after a backup; data is exported;
// labels and shelf tags are previewed and printed. The fake bridge answers as Rust.

const menu = (page: Page) => page.getByRole("navigation", { name: "Main menu" });

async function open(page: Page, entry: RegExp, heading: string) {
  await page.goto("/");
  await menu(page).getByRole("button", { name: entry }).click();
  await expect(page.getByRole("heading", { name: heading, exact: true })).toBeVisible();
}

test("io_01: a product file is checked, then the good rows are imported", async ({ page }) => {
  await open(page, /Import and export/, "Import and export");
  await page.getByRole("button", { name: /^Choose file/ }).click();
  await expect(page.getByText("2 new, 1 changed, 1 left out, of 4 rows.")).toBeVisible();
  await expect(page.getByRole("combobox", { name: "Sale price (needed)" })).toHaveValue("3");
  await expect(page.getByText(/Leave out thousands signs/)).toBeVisible();

  await page.getByRole("checkbox", { name: "Skip rows with errors" }).check();
  await page.getByRole("button", { name: /^Import$/ }).click();
  await expect(page.getByText("2 new, 1 changed, 1 left out.")).toBeVisible();
  await expect(page.getByText(/pre-import/)).toBeVisible();
});

test("io_03: products are exported to Excel", async ({ page }) => {
  await open(page, /Import and export/, "Import and export");
  await page.getByRole("tab", { name: "Export" }).click();
  await page.getByRole("button", { name: /^Export$/ }).click();
  await expect(page.getByText(/rows were saved/)).toBeVisible();
});

test("cat_12/cat_13: labels are previewed and printed, shelf tags laid out on A4", async ({
  page,
}) => {
  await open(page, /Labels and shelf tags/, "Labels and shelf tags");
  await page.getByRole("textbox", { name: "Find product" }).fill("milk");
  await page.getByRole("button", { name: /^Find$/ }).click();
  await page
    .getByRole("button", { name: /^Add Milk/ })
    .first()
    .click();
  await page.getByRole("button", { name: /^Preview$/ }).click();
  await expect(page.getByText("× 1")).toBeVisible();
  await page.getByRole("button", { name: /^Print$/ }).click();
  await expect(page.getByText(/went to the Preview printer/)).toBeVisible();

  await page.getByRole("combobox", { name: "Kind" }).selectOption("shelf_tag");
  await page.getByRole("combobox", { name: "Print on" }).selectOption("a4");
  await page.getByRole("button", { name: /^Preview$/ }).click();
  await expect(page.getByRole("region", { name: "A4 sheet" })).toBeVisible();
  await page.getByRole("button", { name: /^Print$/ }).click();
  await expect(page.getByRole("alert")).toHaveCount(0);
});
