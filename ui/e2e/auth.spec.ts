import { expect, test, type Page } from "@playwright/test";

// Login, lock, users, roles, admin approval and the audit log (USR-01 to USR-09).

async function logIn(page: Page, tile: RegExp, secret: string) {
  await page.getByRole("button", { name: tile }).click();
  await page.locator("input[type=password]").fill(secret);
  await page.getByRole("button", { name: /^Log in/ }).click();
}

function menu(page: Page) {
  return page.getByRole("navigation", { name: "Main menu" });
}

test("usr_01/usr_03: a cashier logs in with a PIN, locks with F12 and unlocks", async ({
  page,
}) => {
  await page.goto("/?shop=signed_out");
  await logIn(page, /Rina Akter/, "1234");
  await expect(page.getByText("Logged in: Rina Akter (Cashier)")).toBeVisible();
  await expect(menu(page).getByRole("button", { name: /Users/ })).toHaveCount(0);

  await page.keyboard.press("F12");
  await expect(page.getByRole("heading", { name: "Till locked" })).toBeVisible();
  await page.getByLabel("PIN or password").fill("1234");
  await page.getByRole("button", { name: /^Unlock$/ }).click();
  await expect(menu(page)).toBeVisible();
});

test("usr_02: five wrong PINs lock the user", async ({ page }) => {
  await page.goto("/?shop=signed_out");
  await page.getByRole("button", { name: /Rina Akter/ }).click();
  for (let attempt = 0; attempt < 5; attempt += 1) {
    await page.getByLabel("PIN").fill("9999");
    await page.getByLabel("PIN").press("Enter");
  }
  await expect(page.getByText(/Too many wrong tries. Try again in \d+ seconds/)).toBeVisible();
});

test("usr_05: a cashier's restore needs an admin's password", async ({ page }) => {
  await page.goto("/?shop=signed_out");
  await logIn(page, /Rina Akter/, "1234");
  await menu(page)
    .getByRole("button", { name: /Backups/ })
    .click();
  await page
    .getByRole("button", { name: /Put this backup back/ })
    .first()
    .click();
  await page.getByRole("button", { name: /Restore and restart/ }).click();

  const dialog = page.getByRole("dialog", { name: "This needs an admin: Restore a backup" });
  await expect(dialog).toBeVisible();
  await dialog.getByLabel("Admin password").fill("shop-owner-1");
  await dialog.getByLabel("Admin password").press("Enter");
  await expect(dialog).toHaveCount(0);
  await expect(page.getByText(/The app will restart/)).toBeVisible();
});

test("usr_06/usr_04: the admin adds a user and a role", async ({ page }) => {
  await page.goto("/");
  await menu(page).getByRole("button", { name: /Users/ }).click();
  await page.getByRole("textbox", { name: "Username" }).fill("karim");
  await page.getByRole("textbox", { name: "Name on screen" }).fill("Karim Hossain");
  await page.getByRole("textbox", { name: "PIN" }).fill("2468");
  await page.getByRole("button", { name: /Save user/ }).click();
  await expect(page.getByRole("cell", { name: "Karim Hossain" })).toBeVisible();

  await menu(page).getByRole("button", { name: /Roles/ }).click();
  await page.getByRole("textbox", { name: "Role name" }).fill("Supervisor");
  await page.getByLabel("Void a completed sale").check();
  await page.getByRole("button", { name: /Save role/ }).click();
  await expect(page.getByRole("cell", { name: "Supervisor" })).toBeVisible();
});

test("usr_08: the audit log shows logins and exports to Excel", async ({ page }) => {
  await page.goto("/?shop=signed_out");
  await logIn(page, /Shop Owner/, "shop-owner-1");
  await menu(page)
    .getByRole("button", { name: /Audit log/ })
    .click();
  await expect(page.getByRole("cell", { name: "Logged in" })).toBeVisible();
  await page.getByRole("button", { name: /Export to Excel/ }).click();
  await expect(page.getByText("Saved 1 actions to the Excel file.")).toBeVisible();
});

test("usr_09: forgot admin password", async ({ page }) => {
  await page.goto("/?shop=signed_out");
  await page.getByRole("button", { name: /Forgot admin password/ }).click();
  await page.getByLabel("Recovery code").fill("7KQ2-M9XD-4RTW-PZ8C-H3NB");
  await page.getByLabel("Admin username").fill("owner");
  await page.getByLabel("New password", { exact: true }).fill("new-owner-pass");
  await page.getByLabel("New password again").fill("new-owner-pass");
  await page.getByRole("button", { name: /Set password/ }).click();
  await expect(page.getByRole("heading", { name: "The new password is set" })).toBeVisible();
  await page.getByRole("button", { name: /I wrote it down/ }).click();
  await logIn(page, /Shop Owner/, "new-owner-pass");
  await expect(menu(page)).toBeVisible();
});
