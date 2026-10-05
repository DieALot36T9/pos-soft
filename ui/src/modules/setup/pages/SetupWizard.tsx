import { ArrowLeft, ArrowRight, Rocket } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { SettingsSnapshot } from "../../../generated/SettingsSnapshot";
import type { SetupStatus } from "../../../generated/SetupStatus";
import type { SetupStep } from "../../../generated/SetupStep";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { getSaved } from "../api";
import { AdminStep } from "../components/AdminStep";
import { BackupStep } from "../components/BackupStep";
import { HardwareStep } from "../components/HardwareStep";
import { MoneyStep } from "../components/MoneyStep";
import { ProductsStep } from "../components/ProductsStep";
import { ShopStep } from "../components/ShopStep";
import { StepList, wizardSteps } from "../components/StepList";
import { TaxStep } from "../components/TaxStep";
import { TillStep } from "../components/TillStep";
import { Welcome } from "../components/Welcome";

type SetupWizardProps = {
  status: SetupStatus;
  // Setup is complete (or a backup was restored): the app shows its next screen.
  onFinished: () => void;
};

type Screen = SetupStep | "welcome" | "done";

// The first-run wizard (SET-01, SET-02): it opens at the first unfinished step.
export function SetupWizard({ status: firstStatus, onFinished }: SetupWizardProps) {
  const { t } = useTranslation("setup");
  const [status, setStatus] = useState(firstStatus);
  const [saved, setSaved] = useState<SettingsSnapshot | null>(null);
  const [screen, setScreen] = useState<Screen>(
    firstStatus.completed.length === 0 ? "welcome" : (firstStatus.next_step ?? "done"),
  );

  // Reload what is saved whenever the step changes, so Back shows saved values.
  useEffect(() => {
    getSaved()
      .then(setSaved)
      .catch(() => undefined);
  }, [screen]);

  if (screen === "welcome") {
    return <Welcome onNewShop={() => setScreen("shop")} onRestored={onFinished} />;
  }

  const step = (offset: number): Screen => {
    const index = wizardSteps.indexOf(screen as SetupStep) + offset;
    return wizardSteps[index] ?? (offset > 0 ? "done" : "welcome");
  };
  const goNext = (newStatus: SetupStatus | null) => {
    if (newStatus) {
      setStatus(newStatus);
    }
    setScreen(step(1));
  };
  const onBack = () => setScreen(step(-1));
  const common = { onSaved: goNext, onBack };

  return (
    <div className="mx-auto flex w-full max-w-5xl gap-6 rounded-ui border border-border bg-panel p-6">
      <StepList current={screen} completed={status.completed} />
      {screen === "done" ? (
        <section className="flex flex-1 flex-col gap-4">
          <h2 className="text-[28px] font-semibold">{t("done.title")}</h2>
          <p>{t("done.intro")}</p>
          {status.completed.includes("hardware") ? null : (
            <Alert kind="warning">{t("done.printerNote")}</Alert>
          )}
          <div className="mt-auto flex justify-end border-t border-border pt-4">
            <ActionButton
              colour="green"
              icon={Rocket}
              label={t("done.start.label")}
              tip={t("done.start.tip")}
              onClick={onFinished}
            />
          </div>
        </section>
      ) : saved === null ? null : screen === "shop" ? (
        <ShopStep saved={saved} {...common} onBack={undefined} />
      ) : screen === "currency" ? (
        <MoneyStep saved={saved} {...common} />
      ) : screen === "tax" ? (
        <TaxStep saved={saved} {...common} />
      ) : screen === "admin" ? (
        status.completed.includes("admin") ? (
          <AdminDone onNext={() => goNext(null)} onBack={onBack} />
        ) : (
          <AdminStep
            onDone={() => goNext({ ...status, completed: [...status.completed, "admin"] })}
            onBack={onBack}
          />
        )
      ) : screen === "till" ? (
        <TillStep saved={saved} {...common} />
      ) : screen === "hardware" ? (
        <HardwareStep saved={saved} {...common} onSkip={() => goNext(null)} />
      ) : screen === "backup" ? (
        <BackupStep saved={saved} {...common} />
      ) : (
        <ProductsStep {...common} />
      )}
    </div>
  );
}

// The admin step after the admin exists: nothing to change here.
function AdminDone({ onNext, onBack }: { onNext: () => void; onBack: () => void }) {
  const { t } = useTranslation("setup");
  return (
    <section className="flex flex-1 flex-col gap-4">
      <h2 className="text-[28px] font-semibold">{t("admin.title")}</h2>
      <Alert kind="done">{t("admin.alreadyDone")}</Alert>
      <div className="mt-auto flex justify-between border-t border-border pt-4">
        <ActionButton
          colour="grey"
          icon={ArrowLeft}
          label={t("nav.back.label")}
          tip={t("nav.back.tip")}
          onClick={onBack}
        />
        <ActionButton
          colour="green"
          icon={ArrowRight}
          label={t("nav.next.label")}
          tip={t("nav.next.tip")}
          onClick={onNext}
        />
      </div>
    </section>
  );
}
