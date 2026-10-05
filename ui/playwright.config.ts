import { defineConfig, devices } from "@playwright/test";

// Screen flows in a real browser against the fake bridge (`pnpm dev:mock`).
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
    command: "pnpm dev:mock",
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
  },
});
