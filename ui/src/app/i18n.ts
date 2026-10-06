import i18next from "i18next";
import { initReactI18next } from "react-i18next";

import { approval } from "../shared/text/en/approval";
import { common } from "../shared/text/en/common";
import { errors } from "../shared/text/en/errors";
import { permissions } from "../shared/text/en/permissions";
import { i18nOptions } from "../shared/text/i18n";
import { en as audit } from "../modules/audit/text/en";
import { en as auth } from "../modules/auth/text/en";
import { en as backup } from "../modules/backup/text/en";
import { en as catalog } from "../modules/catalog/text/en";
import { en as hardware } from "../modules/hardware/text/en";
import { en as settings } from "../modules/settings/text/en";
import { en as setup } from "../modules/setup/text/en";
import { en as tax } from "../modules/tax/text/en";
import { en as users } from "../modules/users/text/en";
import { en as app } from "./text/en";

// Every piece of screen text, by language and then by namespace. Each module's
// text/en.ts is one namespace, named after its folder. A new module adds its line
// here (docs/09 Guides/Adding a Module.md). Another language adds a second block.
export const resources = {
  en: {
    common,
    errors,
    approval,
    permissions,
    app,
    hardware,
    setup,
    settings,
    tax,
    backup,
    auth,
    users,
    audit,
    catalog,
  },
};

// The namespace `t()` uses when a component doesn't name one.
export const defaultNS = "common";

// Starts i18next once, before the first screen. `useTranslation()` then works everywhere.
void i18next
  .use(initReactI18next)
  .init({ ...i18nOptions, resources, defaultNS, ns: Object.keys(resources.en) });

export { i18next as i18n };
