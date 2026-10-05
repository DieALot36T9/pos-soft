// Lint rules for the screens. See docs/09 Guides/Coding Standards.md.
import js from "@eslint/js";
import boundaries from "eslint-plugin-boundaries";
import react from "eslint-plugin-react";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

// Rule R7 (Module Rules), checked by eslint-plugin-boundaries (Reuse Map):
// a module folder imports only shared/, generated/ and itself; shared/ imports only
// shared/ and generated/; app/ may import everything.
const elements = [
  { type: "app", pattern: "src/app" },
  { type: "shared", pattern: "src/shared" },
  { type: "generated", pattern: "src/generated" },
  // Each module folder is its own element, named by its folder.
  { type: "module", pattern: "src/modules/*", capture: ["name"] },
];

const boundaryPolicies = [
  { from: { element: { type: "app" } }, allow: { to: { element: { type: "*" } } } },
  {
    from: { element: { type: "shared" } },
    allow: { to: { element: { types: { anyOf: ["shared", "generated"] } } } },
  },
  {
    from: { element: { type: "module" } },
    allow: { to: { element: { types: { anyOf: ["shared", "generated"] } } } },
  },
  {
    from: { element: { type: "module" } },
    allow: {
      to: { element: { type: "module", captured: { name: "{{ from.element.captured.name }}" } } },
    },
  },
];

export default tseslint.config(
  { ignores: ["dist", "src/generated", "node_modules", "test-results", "playwright-report", "dist-mock"] },
  js.configs.recommended,
  tseslint.configs.recommendedTypeChecked,
  {
    languageOptions: {
      globals: globals.browser,
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    settings: {
      react: { version: "detect" },
      "boundaries/elements": elements,
      // Lets the boundaries rule find .ts/.tsx files from imports written without the extension.
      "import/resolver": { typescript: { project: "./tsconfig.json" } },
    },
    plugins: {
      react,
      "react-hooks": reactHooks,
      boundaries,
    },
    rules: {
      ...react.configs.recommended.rules,
      ...react.configs["jsx-runtime"].rules,
      ...reactHooks.configs.recommended.rules,
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-non-null-assertion": "error",
      // Screen text lives in text/en.ts, never inline (ADR-022).
      "react/jsx-no-literals": ["error", { noStrings: true, ignoreProps: true }],
      "boundaries/dependencies": ["error", { default: "disallow", policies: boundaryPolicies }],
    },
  },
  {
    files: ["**/*.test.{ts,tsx}", "src/test-setup.ts"],
    rules: {
      "react/jsx-no-literals": "off",
    },
  },
  {
    files: ["eslint.config.js", "vite.config.ts", "playwright.config.ts", "e2e/**"],
    languageOptions: { globals: globals.node },
  },
  {
    files: ["eslint.config.js"],
    ...tseslint.configs.disableTypeChecked,
  },
);
