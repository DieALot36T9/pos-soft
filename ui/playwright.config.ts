import { defineConfig, devices } from "@playwright/test";

// Screen flows in a real browser against the fake bridge. The screens are built once
// in mock mode and served as they are (`pnpm e2e:serve`), so no page reloads mid-test.
// Run with `pnpm e2e`. See docs/08 Testing/Test Strategy.md.
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: process.env.CI ? "github" : "list",
  use: {
    baseURL: "http://localhost:1420",
    // The shop screen size (Design System).
    viewport: { width: 1366, height: 768 },
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "pnpm e2e:serve",
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
  },
});
