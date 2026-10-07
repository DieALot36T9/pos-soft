import type { TFunction } from "i18next";

import { en } from "../text/en";

type CellCode = keyof typeof en.cells;

// True when the screen has words for this problem code.
function isCellCode(code: string): code is CellCode {
  return Object.hasOwn(en.cells, code);
}

// A problem code from the preview, in plain words. Example: "money_not_a_number".
export function problemText(code: string, t: TFunction<"io">): string {
  return isCellCode(code) && code !== "other" ? t(`cells.${code}`) : t("cells.other", { code });
}
