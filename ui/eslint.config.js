// Lint rules for the screens. See docs/09 Guides/Coding Standards.md.
import path from "node:path";

import js from "@eslint/js";
import react from "eslint-plugin-react";
import reactHooks from "eslint-plugin-react-hooks";
import globals from "globals";
import tseslint from "typescript-eslint";

const sourceFolder = path.resolve(import.meta.dirname, "src");

// Which top-level part of src/ a file is in: "modules/catalog", "shared", "app", …
function areaOf(file) {
  const relative = path.relative(sourceFolder, file).split(path.sep);
  if (relative[0] === "modules") {
    return `modules/${relative[1]}`;
  }
  return relative[0];
}

// Rule R7 (Module Rules): a module folder imports only shared/, generated/ and itself.
// shared/ imports only shared/ and generated/.
const moduleBoundaries = {
  meta: { type: "problem", schema: [] },
  create(context) {
    const fromArea = areaOf(context.filename);
    return {
      ImportDeclaration(node) {
        const target = node.source.value;
        if (typeof target !== "string" || !target.startsWith(".")) {
          return;
        }
        const toArea = areaOf(path.resolve(path.dirname(context.filename), target));
        const isOwnArea = toArea === fromArea;
        // Files outside src/ (such as the shared money examples) are not screen code.
        const isAlwaysAllowed = toArea === "shared" || toArea === "generated" || toArea === "..";
        const fromModule = fromArea.startsWith("modules/");
        const fromShared = fromArea === "shared";
        if ((fromModule || fromShared) && !isOwnArea && !isAlwaysAllowed) {
          context.report({
            node,
            message: `${fromArea} may not import from ${toArea}. Move shared code to src/shared/.`,
          });
        }
      },
    };
  },
};

export default tseslint.config(
  { ignores: ["dist", "src/generated", "node_modules"] },
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
    settings: { react: { version: "detect" } },
    plugins: {
      react,
      "react-hooks": reactHooks,
      local: { rules: { "module-boundaries": moduleBoundaries } },
    },
    rules: {
      ...react.configs.recommended.rules,
      ...react.configs["jsx-runtime"].rules,
      ...reactHooks.configs.recommended.rules,
      "@typescript-eslint/no-explicit-any": "error",
      "@typescript-eslint/no-non-null-assertion": "error",
      // Screen text lives in text/en.ts, never inline (ADR-022).
      "react/jsx-no-literals": ["error", { noStrings: true, ignoreProps: true }],
      "local/module-boundaries": "error",
    },
  },
  {
    files: ["**/*.test.{ts,tsx}", "src/test-setup.ts"],
    rules: {
      "react/jsx-no-literals": "off",
    },
  },
  {
    files: ["eslint.config.js", "vite.config.ts"],
    languageOptions: { globals: globals.node },
  },
  {
    files: ["eslint.config.js"],
    ...tseslint.configs.disableTypeChecked,
  },
);
