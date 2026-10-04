// @vitest-environment node
// Proves the lint rule for rule R7 (Module Rules) works: `pnpm lint` fails when a module
// folder imports another module folder. It writes two small throw-away modules, lints
// them with the real eslint.config.js, and deletes them again.
import { mkdirSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";

import { ESLint } from "eslint";
import { afterAll, beforeAll, describe, expect, it } from "vitest";

const uiFolder = path.resolve(import.meta.dirname, "../..");
const modulesFolder = path.join(uiFolder, "src/modules");
const moduleA = path.join(modulesFolder, "lint_fixture_a");
const moduleB = path.join(modulesFolder, "lint_fixture_b");

function write(file: string, code: string) {
  mkdirSync(path.dirname(file), { recursive: true });
  writeFileSync(file, code);
}

// Lints one file and returns the messages of the boundaries rule only.
async function boundaryProblems(file: string): Promise<string[]> {
  const eslint = new ESLint({ cwd: uiFolder });
  const [result] = await eslint.lintFiles([file]);
  return (result?.messages ?? [])
    .filter((message) => message.ruleId === "boundaries/dependencies")
    .map((message) => message.message);
}

describe("module boundaries (eslint-plugin-boundaries)", () => {
  beforeAll(() => {
    write(path.join(moduleB, "thing.ts"), "export const thing = 1;\n");
    write(path.join(moduleA, "own.ts"), "export const own = 2;\n");
    write(
      path.join(moduleA, "good.ts"),
      [
        'import type { AppInfo } from "../../generated/AppInfo";',
        'import { cn } from "../../shared/lib/utils";',
        'import { own } from "./own";',
        "export const good: AppInfo | string = cn(String(own));",
        "",
      ].join("\n"),
    );
    write(
      path.join(moduleA, "bad.ts"),
      'import { thing } from "../lint_fixture_b/thing";\nexport const bad = thing;\n',
    );
  });

  afterAll(() => {
    rmSync(moduleA, { recursive: true, force: true });
    rmSync(moduleB, { recursive: true, force: true });
  });

  it("lets a module import shared/, generated/ and itself", async () => {
    expect(await boundaryProblems(path.join(moduleA, "good.ts"))).toEqual([]);
  }, 60_000);

  it("refuses a module importing another module", async () => {
    const problems = await boundaryProblems(path.join(moduleA, "bad.ts"));
    expect(problems).toHaveLength(1);
    expect(problems[0]).toContain('name="lint_fixture_b"');
  }, 60_000);
});
