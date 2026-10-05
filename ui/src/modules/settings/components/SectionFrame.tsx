import { Save } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";

type SectionFrameProps = {
  title: string;
  children: ReactNode;
  busy: boolean;
  done: boolean;
  problem: string | null;
  onSave: () => void;
};

// One settings section: its fields, the result of saving, and Save bottom-right.
export function SectionFrame({ title, children, busy, done, problem, onSave }: SectionFrameProps) {
  const { t } = useTranslation("settings");
  return (
    <section
      aria-labelledby="section-title"
      className="flex flex-1 flex-col gap-4 rounded-ui border border-border bg-panel p-4"
    >
      <h3 id="section-title" className="text-xl font-semibold">
        {title}
      </h3>
      <div className="flex flex-col gap-4">{children}</div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      {done && !problem ? <Alert kind="done">{t("saved")}</Alert> : null}
      <div className="flex justify-end border-t border-border pt-4">
        <ActionButton
          colour="green"
          icon={Save}
          label={t("save.label")}
          tip={t("save.tip")}
          disabledReason={busy ? t("save.busy") : undefined}
          onClick={onSave}
        />
      </div>
    </section>
  );
}
