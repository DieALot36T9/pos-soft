import { createContext, useContext, type ReactNode } from "react";

// Languages the app has. Only English exists today (ADR-022).
export type Language = "en";

// All the text of one module, in every language it has. English is always there.
// A new language file must match the English shape exactly, or the build fails.
export type TextBundle<T> = { en: T } & Partial<Record<Language, T>>;

const LanguageContext = createContext<Language>("en");

// Sets the language for everything inside it. The language comes from settings.
export function LanguageProvider({
  language,
  children,
}: {
  language: Language;
  children: ReactNode;
}) {
  return <LanguageContext value={language}>{children}</LanguageContext>;
}

// Gives a component its module's text in the current language.
// Example: const text = useText(appText); then text.checkButton.label
export function useText<T>(bundle: TextBundle<T>): T {
  const language = useContext(LanguageContext);
  return bundle[language] ?? bundle.en;
}
