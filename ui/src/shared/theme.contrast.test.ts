// Every text colour on its background must reach WCAG AAA contrast (7 : 1 or more).
// See "Always readable" in docs/07 UI Design/Design System.md.
import { readFileSync } from "node:fs";
import path from "node:path";

import { describe, expect, it } from "vitest";

const themeFile = readFileSync(path.join(import.meta.dirname, "theme.css"), "utf8");

// Reads a colour token such as --green from theme.css.
function token(name: string): string {
  const match = new RegExp(`--${name}:\\s*(#[0-9A-Fa-f]{6})`).exec(themeFile);
  if (!match?.[1]) {
    throw new Error(`token --${name} is missing from theme.css`);
  }
  return match[1];
}

// Relative luminance, as defined by WCAG 2.
function luminance(hex: string): number {
  const channels = [1, 3, 5].map((start) => parseInt(hex.slice(start, start + 2), 16) / 255);
  const linear = channels.map((value) =>
    value <= 0.03928 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4,
  );
  const [red = 0, green = 0, blue = 0] = linear;
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue;
}

function contrast(foreground: string, background: string): number {
  const lighter = Math.max(luminance(foreground), luminance(background));
  const darker = Math.min(luminance(foreground), luminance(background));
  return (lighter + 0.05) / (darker + 0.05);
}

const white = "#FFFFFF";

// [what is shown, text colour, background colour]
const pairs: [string, string, string][] = [
  ["main text on the page", token("text"), token("bg")],
  ["main text on a panel", token("text"), token("panel")],
  ["secondary text on the page", token("text-muted"), token("bg")],
  ["secondary text on a panel", token("text-muted"), token("panel")],
  ["tooltip text", white, token("text")],
  ["green button", white, token("green")],
  ["red button", white, token("red")],
  ["blue button", white, token("blue")],
  ["purple button", white, token("purple")],
  ["teal button", white, token("teal")],
  ["grey button", white, token("grey")],
  ["amber button (dark text)", token("text"), token("amber-fill")],
  ["green on soft green", token("green"), token("green-soft")],
  ["red on soft red", token("red"), token("red-soft")],
  ["amber on soft amber", token("amber"), token("amber-soft")],
  ["blue on soft blue", token("blue"), token("blue-soft")],
  ["purple on soft purple", token("purple"), token("purple-soft")],
  ["teal on soft teal", token("teal"), token("teal-soft")],
  ["grey on soft grey", token("grey"), token("grey-soft")],
];

describe("colour contrast", () => {
  it.each(pairs)("%s is at least 7 : 1", (_name, foreground, background) => {
    expect(contrast(foreground, background)).toBeGreaterThanOrEqual(7);
  });
});
