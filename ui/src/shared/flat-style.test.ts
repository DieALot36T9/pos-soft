// Guards the flat look (Design System): no shadow, gradient, blur, animation or
// transition classes anywhere in the screens' source code.
import { readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const sourceFolder = path.join(import.meta.dirname, "..");

function sourceFiles(folder: string): string[] {
  const files: string[] = [];
  for (const name of readdirSync(folder)) {
    const full = path.join(folder, name);
    if (statSync(full).isDirectory()) {
      files.push(...sourceFiles(full));
    } else if (/\.(tsx?|css)$/.test(name) && !name.endsWith(".test.ts")) {
      files.push(full);
    }
  }
  return files;
}

const forbidden =
  /\b(shadow-(?!none)|drop-shadow|bg-gradient|bg-linear|bg-radial|blur-|backdrop-blur|animate-|transition)/;

describe("flat look", () => {
  it.each(sourceFiles(sourceFolder))("%s has no shadow, gradient, blur or animation", (file) => {
    const contents = readFileSync(file, "utf8");
    // theme.css names the removed utilities on purpose, to switch them off.
    const checked = file.endsWith("theme.css")
      ? contents.replace(/@theme \{[^}]*\}/, "")
      : contents;
    expect(checked).not.toMatch(forbidden);
  });
});
