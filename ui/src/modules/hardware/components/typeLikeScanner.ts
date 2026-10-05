// Real scanners act like a keyboard: they type the code very fast and press Enter.
// This does the same to a text box, so the screen can't tell it from a real scan.

// The pause between two characters. Scanners type a character every few milliseconds.
export const SCANNER_GAP_MS = 8;

// React watches the browser's own value setter, so the value must be set through it.
// It is called with the text box as `this` below.
// eslint-disable-next-line @typescript-eslint/unbound-method
const setNativeValue = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set;

const pause = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

// Presses one key on the box, the way the keyboard does.
function press(input: HTMLInputElement, key: string) {
  input.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
}

// Types the code into the box one character at a time, then presses Enter.
export async function typeLikeScanner(
  input: HTMLInputElement,
  code: string,
  gapMs: number = SCANNER_GAP_MS,
): Promise<void> {
  input.focus();
  for (const character of code) {
    press(input, character);
    setNativeValue?.call(input, input.value + character);
    input.dispatchEvent(new Event("input", { bubbles: true }));
    await pause(gapMs);
  }
  press(input, "Enter");
}
