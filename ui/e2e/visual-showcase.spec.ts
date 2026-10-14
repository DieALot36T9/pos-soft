import { expect, test, type Page } from "@playwright/test";

const SCREENSHOT_DIR =
  "/Users/wisp/.gemini/antigravity-ide/brain/ed240d70-685e-44eb-b1ea-62de4000c7c0/screenshots";

async function openTill(page: Page) {
  await page.goto("/");
  await page
    .getByRole("navigation", { name: "Main menu" })
    .getByRole("button", { name: /^Checkout$/ })
    .click();

  await expect(page.getByText("Open a shift to start selling")).toBeVisible();
  await page.getByRole("button", { name: /^Open shift$/ }).click();
  const dialog = page.getByRole("dialog", { name: "Open a shift" });
  await expect(dialog).toBeVisible();
  await dialog.getByRole("textbox", { name: "Opening float" }).fill("1000");
  await dialog.getByRole("button", { name: /Open shift/ }).click();
  await expect(page.getByText("Scan an item to start the sale.")).toBeVisible();
}

test.describe.serial("Visual Showcase E2E", () => {
  test.use({ viewport: { width: 1366, height: 768 } });

  test("01: Dashboard and Stock Alerts", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "Stock alerts" })).toBeVisible();
    await page.screenshot({ path: `${SCREENSHOT_DIR}/01_dashboard.png`, fullPage: true });
  });

  test("02: Till Checkout & Sales Flow", async ({ page }) => {
    await openTill(page);
    await page.screenshot({ path: `${SCREENSHOT_DIR}/02_till_empty.png` });

    // Scan Milk
    await page.keyboard.type("8901234567890");
    await page.keyboard.press("Enter");
    await expect(page.getByRole("row").filter({ hasText: "Milk 1L" })).toBeVisible();

    // Adjust Quantity
    await page.keyboard.press("F4");
    await page.getByRole("textbox", { name: "Quantity" }).fill("3");
    await page.keyboard.press("Enter");

    await page.screenshot({ path: `${SCREENSHOT_DIR}/03_till_active_cart.png` });

    // Open Payment Modal
    await page.keyboard.press("F10");
    const payment = page.getByRole("dialog", { name: "Payment" });
    await expect(payment).toBeVisible();
    await payment.getByRole("textbox", { name: "Cash" }).fill("500");
    await page.screenshot({ path: `${SCREENSHOT_DIR}/04_till_payment_modal.png` });

    // Complete Sale
    await page.keyboard.press("Enter");
    await expect(page.getByText(/Sale complete/)).toBeVisible();
    await page.screenshot({ path: `${SCREENSHOT_DIR}/05_till_sale_complete.png` });
  });

  test("03: Pricing Page (Price Lists & Tiers)", async ({ page }) => {
    await page.goto("/");
    await page
      .getByRole("navigation", { name: "Main menu" })
      .getByRole("button", { name: /^Prices and promotions$/ })
      .click();
    await expect(
      page.getByRole("heading", { name: "Prices and promotions", exact: true }),
    ).toBeVisible();
    await page.screenshot({ path: `${SCREENSHOT_DIR}/06_pricing_panel.png`, fullPage: true });

    // Click Tiers tab
    const tiersTab = page.getByRole("tab", { name: "Tiers" });
    if (await tiersTab.isVisible().catch(() => false)) {
      await tiersTab.click();
      await page.screenshot({ path: `${SCREENSHOT_DIR}/07_pricing_tiers.png`, fullPage: true });
    }
  });

  test("04: Purchasing Workspace Panels", async ({ page }) => {
    await page.goto("/");
    await page
      .getByRole("navigation", { name: "Main menu" })
      .getByRole("button", { name: /^Purchasing$/ })
      .click();
    await expect(page.getByRole("heading", { name: "Purchasing", exact: true })).toBeVisible();
    await page.screenshot({
      path: `${SCREENSHOT_DIR}/08_purchasing_suppliers.png`,
      fullPage: true,
    });

    // Click Orders tab
    const ordersTab = page.getByRole("tab", { name: "Purchase orders" });
    if (await ordersTab.isVisible().catch(() => false)) {
      await ordersTab.click();
      await page.screenshot({ path: `${SCREENSHOT_DIR}/09_purchasing_orders.png`, fullPage: true });
    }

    // Click Receive tab
    const receiveTab = page.getByRole("tab", { name: "Receive goods" });
    if (await receiveTab.isVisible().catch(() => false)) {
      await receiveTab.click();
      await page.screenshot({
        path: `${SCREENSHOT_DIR}/10_purchasing_receive.png`,
        fullPage: true,
      });
    }

    // Click Payables tab
    const payablesTab = page.getByRole("tab", { name: "What we owe" });
    if (await payablesTab.isVisible().catch(() => false)) {
      await payablesTab.click();
      await page.screenshot({
        path: `${SCREENSHOT_DIR}/11_purchasing_payables.png`,
        fullPage: true,
      });
    }
  });

  test("05: Import / Export Wizard", async ({ page }) => {
    await page.goto("/");
    await page
      .getByRole("navigation", { name: "Main menu" })
      .getByRole("button", { name: /^Import and export$/ })
      .click();
    await expect(
      page.getByRole("heading", { name: "Import and export", exact: true }),
    ).toBeVisible();
    await page.screenshot({ path: `${SCREENSHOT_DIR}/12_io_wizard.png`, fullPage: true });
  });

  test("06: Settings & Payment Methods", async ({ page }) => {
    await page.goto("/");
    await page
      .getByRole("navigation", { name: "Main menu" })
      .getByRole("button", { name: /^Settings$/ })
      .click();
    await expect(page.getByRole("heading", { name: "Settings", exact: true })).toBeVisible();

    // Click Payment methods section in settings sidebar
    const paymentsSection = page.getByRole("button", { name: "Payment methods" });
    if (await paymentsSection.isVisible().catch(() => false)) {
      await paymentsSection.click();
      await page.screenshot({
        path: `${SCREENSHOT_DIR}/13_settings_payment_methods.png`,
        fullPage: true,
      });
    }
  });

  test("07: Reports View", async ({ page }) => {
    await page.goto("/");
    await page
      .getByRole("navigation", { name: "Main menu" })
      .getByRole("button", { name: /^Reports$/ })
      .click();
    await expect(page.getByRole("heading", { name: "Reports", exact: true })).toBeVisible();
    await page.screenshot({ path: `${SCREENSHOT_DIR}/14_reports_view.png`, fullPage: true });
  });

  test("08: Stock Counts (CountingScreen & VarianceReview)", async ({ page }) => {
    await page.goto("/");
    await page
      .getByRole("navigation", { name: "Main menu" })
      .getByRole("button", { name: /^Stock counts$/ })
      .click();
    await expect(page.getByRole("heading", { name: "Stock counts", exact: true })).toBeVisible();

    // Create a new count
    await page.getByRole("textbox", { name: "Name" }).fill("Quarterly Store Audit");
    await page.getByRole("button", { name: /Start count/ }).click();
    await expect(page.getByRole("heading", { name: /Quarterly Store Audit/ })).toBeVisible();

    // Capture CountingScreen
    await page.screenshot({ path: `${SCREENSHOT_DIR}/15_counting_screen.png`, fullPage: true });

    // Review differences (VarianceReview)
    const reviewBtn = page.getByRole("button", { name: /Review differences/ });
    if (await reviewBtn.isVisible().catch(() => false)) {
      await reviewBtn.click();
      await expect(page.getByRole("heading", { name: /Quarterly Store Audit/ })).toBeVisible();
      await page.screenshot({ path: `${SCREENSHOT_DIR}/16_variance_review.png`, fullPage: true });
    }
  });

  test("09: Setup Wizard (Welcome & StepList)", async ({ page }) => {
    await page.goto("/?shop=fresh");
    await expect(page.getByRole("heading", { name: "Welcome to Supershop POS" })).toBeVisible();
    await page.screenshot({ path: `${SCREENSHOT_DIR}/17_setup_welcome.png`, fullPage: true });

    await page.getByRole("button", { name: /Set up a new shop/ }).click();
    await expect(page.getByRole("heading", { name: "Your shop" })).toBeVisible();
    await page.screenshot({
      path: `${SCREENSHOT_DIR}/18_setup_steplist_stepframe.png`,
      fullPage: true,
    });
  });
});
