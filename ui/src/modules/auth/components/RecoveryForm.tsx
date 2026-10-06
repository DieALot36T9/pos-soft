import { ArrowLeft, Check, KeyRound } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { useSave } from "../../../shared/bridge/useSave";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { recoverAdmin } from "../api";

// "Forgot admin password" (USR-09): the recovery code sets a new password for one admin.
// The new recovery code is shown once.
export function RecoveryForm({ onDone }: { onDone: () => void }) {
  const { t } = useTranslation("auth");
  const [code, setCode] = useState("");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [passwordAgain, setPasswordAgain] = useState("");
  const [newCode, setNewCode] = useState<string | null>(null);
  const save = useSave();

  const submit = () =>
    save.run(
      () => recoverAdmin(code, username, password, passwordAgain),
      (done) => setNewCode(done.recovery_code),
    );

  if (newCode) {
    return (
      <section
        aria-labelledby="recovery-done-title"
        className="mx-auto flex w-full max-w-md flex-col gap-4 rounded-ui border border-border bg-panel p-6"
      >
        <h2 id="recovery-done-title" className="text-[28px] font-semibold">
          {t("recovery.doneTitle")}
        </h2>
        <Alert kind="warning">{t("recovery.doneText")}</Alert>
        <p className="rounded-ui border border-border bg-bg p-3 text-center font-mono text-2xl">
          {newCode}
        </p>
        <div className="flex justify-end">
          <ActionButton
            colour="green"
            icon={Check}
            label={t("recovery.doneButton.label")}
            tip={t("recovery.doneButton.tip")}
            onClick={onDone}
          />
        </div>
      </section>
    );
  }

  return (
    <section
      aria-labelledby="recovery-title"
      className="mx-auto flex w-full max-w-md flex-col gap-4 rounded-ui border border-border bg-panel p-6"
    >
      <h2 id="recovery-title" className="text-[28px] font-semibold">
        {t("recovery.title")}
      </h2>
      <p className="text-text-muted">{t("recovery.intro")}</p>
      <Field
        label={t("recovery.code.label")}
        tip={t("recovery.code.tip")}
        value={code}
        onChange={setCode}
        error={save.errorFor("code")}
      />
      <Field
        label={t("recovery.username.label")}
        tip={t("recovery.username.tip")}
        value={username}
        onChange={setUsername}
        error={save.errorFor("username")}
      />
      <Field
        label={t("recovery.password.label")}
        tip={t("recovery.password.tip")}
        value={password}
        onChange={setPassword}
        error={save.errorFor("password")}
        secret
      />
      <Field
        label={t("recovery.passwordAgain.label")}
        tip={t("recovery.passwordAgain.tip")}
        value={passwordAgain}
        onChange={setPasswordAgain}
        error={save.errorFor("password_again")}
        secret
      />
      {save.problem && !["code", "username", "password", "password_again"].some(save.errorFor) ? (
        <Alert kind="problem">{save.problem}</Alert>
      ) : null}
      <div className="flex justify-between">
        <ActionButton
          colour="grey"
          icon={ArrowLeft}
          label={t("recovery.back.label")}
          tip={t("recovery.back.tip")}
          onClick={onDone}
        />
        <ActionButton
          colour="green"
          icon={KeyRound}
          label={t("recovery.save.label")}
          tip={t("recovery.save.tip")}
          onClick={submit}
        />
      </div>
    </section>
  );
}
