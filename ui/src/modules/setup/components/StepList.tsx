import { CheckCircle, Circle, CircleDot } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { SetupStep } from "../../../generated/SetupStep";

export const wizardSteps: SetupStep[] = [
  "shop",
  "currency",
  "tax",
  "admin",
  "till",
  "hardware",
  "backup",
  "products",
];

type StepListProps = {
  current: SetupStep | "done";
  completed: SetupStep[];
};

const PROGRESS_TITLE = "Setup Progress";

export function StepList({ current, completed }: StepListProps) {
  const { t } = useTranslation("setup");
  return (
    <nav aria-label={t("steps.listLabel")} className="w-60 shrink-0 border-r border-border pr-4">
      <div className="text-xs font-bold uppercase tracking-wider text-text-muted mb-2 px-2">
        {PROGRESS_TITLE}
      </div>
      <ol className="flex flex-col gap-1.5">
        {wizardSteps.map((step, index) => {
          const isCurrent = step === current;
          const isDone = completed.includes(step);
          const Icon = isDone ? CheckCircle : isCurrent ? CircleDot : Circle;
          return (
            <li
              key={step}
              aria-current={isCurrent ? "step" : undefined}
              className={
                "flex items-center gap-2.5 rounded-ui border px-3 py-2 text-sm " +
                (isCurrent
                  ? "border-blue bg-blue-soft font-bold text-blue"
                  : isDone
                    ? "border-green/30 bg-green-soft/40 font-medium text-text"
                    : "border-transparent text-text-muted")
              }
            >
              <Icon
                aria-hidden="true"
                size={18}
                className={isDone ? "text-green" : isCurrent ? "text-blue" : "text-text-muted"}
              />
              <span>{t("steps.numbered", { number: index + 1, name: t(`steps.${step}`) })}</span>
              {isDone ? <span className="sr-only">{t("steps.doneMark")}</span> : null}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
