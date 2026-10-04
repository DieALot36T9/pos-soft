import type { InitOptions } from "i18next";

// Settings every i18next instance uses (ADR-022). Only English exists today.
export const i18nOptions = {
  lng: "en",
  fallbackLng: "en",
  // React already makes text safe to show, so i18next must not escape it a second time.
  interpolation: { escapeValue: false },
  // All text is bundled with the app, so i18next can be ready before the first screen.
  initAsync: false,
} satisfies InitOptions;
