import { Check, Printer } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { AdminCreated } from "../../../generated/AdminCreated";
import { useSave } from "../../../shared/bridge/useSave";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { printWindow, saveAdmin } from "../api";
import { StepFrame } from "./StepFrame";

type AdminStepProps = {
  // Called after the recovery code was shown and the owner says it is saved.
  onDone: () => void;
  onBack?: () => void;
};

// Step 4: the first admin, then the recovery code shown once (SET-08, USR-09).
export function AdminStep({ onDone, onBack }: AdminStepProps) {
  const { t } = useTranslation("setup");
  const [username, setUsername] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");
  const [passwordAgain, setPasswordAgain] = useState("");
  const [created, setCreated] = useState<AdminCreated | null>(null);
  const save = useSave();

  if (created) {
    return (
      <section aria-labelledby="code-title" className="flex flex-col gap-4">
        <h2 id="code-title" className="text-[28px] font-semibold">
          {t("admin.codeTitle")}
        </h2>
        <Alert kind="warning">{t("admin.codeIntro")}</Alert>
        <p
          data-print-area=""
          data-testid="recovery-code"
          className="rounded-ui border border-border bg-panel p-4 text-center font-mono text-[28px] font-semibold"
        >
          {created.recovery_code}
        </p>
        <div className="flex justify-between border-t border-border pt-4">
          <ActionButton
            colour="blue"
            icon={Printer}
            label={t("admin.print.label")}
            tip={t("admin.print.tip")}
            onClick={() => void printWindow().catch(() => undefined)}
          />
          <ActionButton
            colour="green"
            icon={Check}
            label={t("admin.saved.label")}
            tip={t("admin.saved.tip")}
            onClick={onDone}
          />
        </div>
      </section>
    );
  }

  const next = () =>
    save.run(
      () =>
        saveAdmin({
          username,
          display_name: displayName,
          password,
          password_again: passwordAgain,
        }),
      setCreated,
    );

  return (
    <StepFrame
      title={t("admin.title")}
      intro={t("admin.intro")}
      busy={save.busy}
      problem={save.problem}
      onBack={onBack}
      onNext={next}
    >
      <div className="grid grid-cols-2 gap-4">
        <Field
          label={t("admin.username.label")}
          tip={t("admin.username.tip")}
          value={username}
          onChange={setUsername}
          error={save.errorFor("username")}
        />
        <Field
          label={t("admin.displayName.label")}
          tip={t("admin.displayName.tip")}
          value={displayName}
          onChange={setDisplayName}
          error={save.errorFor("display_name")}
        />
        <Field
          label={t("admin.password.label")}
          tip={t("admin.password.tip")}
          value={password}
          onChange={setPassword}
          error={save.errorFor("password")}
          secret
        />
        <Field
          label={t("admin.passwordAgain.label")}
          tip={t("admin.passwordAgain.tip")}
          value={passwordAgain}
          onChange={setPasswordAgain}
          error={save.errorFor("password_again")}
          secret
        />
      </div>
    </StepFrame>
  );
}
