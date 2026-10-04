// Tells TypeScript every namespace and key, so `t("wrong.key")` or
// `useTranslation("no_such_module")` is a build error (ADR-022).
import type { defaultNS, resources } from "./i18n";

declare module "i18next" {
  interface CustomTypeOptions {
    defaultNS: typeof defaultNS;
    resources: (typeof resources)["en"];
  }
}
