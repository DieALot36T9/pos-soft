import { useState } from "react";
import { useTranslation } from "react-i18next";

import { useSave } from "../../../shared/bridge/useSave";
import { Alert } from "../../../shared/ui/Alert";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { setDetailedLogging } from "../api";
import type { SectionProps } from "./types";

// The detailed-logging switch: it works at once, without a restart (BAK-07).
export function LogsSection({ saved }: Pick<SectionProps, "saved">) {
  const { t } = useTranslation("settings");
  const [detailed, setDetailed] = useState(saved.logging.detailed);
  const save = useSave();

  const change = (on: boolean) => {
    setDetailed(on);
    save.run(() => setDetailedLogging(on));
  };

  return (
    <section
      aria-labelledby="logs-title"
      className="flex flex-1 flex-col gap-4 rounded-ui border border-border bg-panel p-4"
    >
      <h3 id="logs-title" className="text-xl font-semibold">
        {t("sections.logs")}
      </h3>
      <CheckBox
        label={t("logs.detailed.label")}
        tip={t("logs.detailed.tip")}
        checked={detailed}
        onChange={change}
      />
      <p className="text-text-muted">{t("logs.note")}</p>
      {save.problem ? <Alert kind="problem">{save.problem}</Alert> : null}
      {save.done ? <Alert kind="done">{t("logs.switched")}</Alert> : null}
    </section>
  );
}
