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

// The list on the left that shows where you are in setup (Setup Wizard).
export function StepList({ current, completed }: StepListProps) {
  const { t } = useTranslation("setup");
  return (
    <nav aria-label={t("steps.listLabel")} className="w-56 shrink-0 border-r border-border pr-4">
      <ol className="flex flex-col gap-1">
        {wizardSteps.map((step, index) => {
          const isCurrent = step === current;
          const isDone = completed.includes(step);
          const Icon = isDone ? CheckCircle : isCurrent ? CircleDot : Circle;
          return (
            <li
              key={step}
              aria-current={isCurrent ? "step" : undefined}
              className={
                "flex items-center gap-2 rounded-ui px-2 py-2 " +
                (isCurrent ? "bg-blue-soft font-semibold text-blue" : "")
              }
            >
              <Icon aria-hidden="true" size={18} className={isDone ? "text-green" : undefined} />
              <span>{t("steps.numbered", { number: index + 1, name: t(`steps.${step}`) })}</span>
              {isDone ? <span className="sr-only">{t("steps.doneMark")}</span> : null}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
