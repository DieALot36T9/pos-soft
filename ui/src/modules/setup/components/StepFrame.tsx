import { ArrowLeft, ArrowRight, SkipForward } from "lucide-react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";

type StepFrameProps = {
  title: string;
  intro: string;
  children: ReactNode;
  busy: boolean;
  problem: string | null;
  onBack?: () => void;
  onNext: () => void;
  onSkip?: () => void;
};

export function StepFrame({
  title,
  intro,
  children,
  busy,
  problem,
  onBack,
  onNext,
  onSkip,
}: StepFrameProps) {
  const { t } = useTranslation("setup");
  return (
    <section aria-labelledby="step-title" className="flex flex-1 flex-col gap-5">
      <div className="border-b border-border pb-3">
        <h2 id="step-title" className="text-[28px] font-bold text-text">
          {title}
        </h2>
        <p className="text-text-muted mt-0.5">{intro}</p>
      </div>
      <div className="flex flex-col gap-4">{children}</div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      <div className="mt-auto flex items-center justify-between border-t border-border pt-4">
        <div>
          {onBack ? (
            <ActionButton
              colour="grey"
              icon={ArrowLeft}
              label={t("nav.back.label")}
              tip={t("nav.back.tip")}
              onClick={onBack}
            />
          ) : null}
        </div>
        <div className="flex gap-2">
          {onSkip ? (
            <ActionButton
              colour="grey"
              icon={SkipForward}
              label={t("nav.skip.label")}
              tip={t("nav.skip.tip")}
              onClick={onSkip}
            />
          ) : null}
          <ActionButton
            colour="green"
            icon={ArrowRight}
            label={t("nav.next.label")}
            tip={t("nav.next.tip")}
            disabledReason={busy ? t("nav.saving") : undefined}
            onClick={onNext}
          />
        </div>
      </div>
    </section>
  );
}
