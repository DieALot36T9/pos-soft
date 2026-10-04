import { common, type CommonText } from "./en/common";
import { errors, type ErrorText } from "./en/errors";
import type { TextBundle } from "./useText";

// Shared text in every language. Only English exists today.
export const commonText: TextBundle<CommonText> = { en: common };
export const errorText: TextBundle<ErrorText> = { en: errors };

export { LanguageProvider, useText, type Language, type TextBundle } from "./useText";
