import { Lock, LogOut, ShieldCheck, Unlock } from "lucide-react";
import { useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

import type { SecretCheck } from "../../../generated/SecretCheck";
import type { SessionView } from "../../../generated/SessionView";
import { errorMessage } from "../../../shared/bridge";
import { secondsUntil } from "../../../shared/format/time";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { logout, unlock, unlockByAdmin } from "../api";

type LockScreenProps = {
  session: SessionView;
  onUnlocked: () => void;
  onLoggedOut: () => void;
};

// The full-screen lock (USR-03): the user who locked it unlocks with their PIN or
// password, or an admin unlocks to help. "Switch user" logs the user out.
export function LockScreen({ session, onUnlocked, onLoggedOut }: LockScreenProps) {
  const { t } = useTranslation("auth");
  const { t: tErrors } = useTranslation("errors");
  const name = session.display_name;
  const [secret, setSecret] = useState("");
  const [adminName, setAdminName] = useState("");
  const [adminPassword, setAdminPassword] = useState("");
  const [problem, setProblem] = useState<string | null>(null);

  const answer = (result: SecretCheck) => {
    setSecret("");
    setAdminPassword("");
    switch (result.kind) {
      case "accepted":
        onUnlocked();
        break;
      case "wrong_secret":
        setProblem(t("lock.wrong", { left: result.attempts_left }));
        break;
      case "locked":
        setProblem(t("lock.locked", { seconds: secondsUntil(result.until_ms) }));
        break;
      case "not_allowed":
        setProblem(t("lock.adminNotAllowed"));
        break;
    }
  };
  const failed = (error: unknown) => setProblem(errorMessage(error, tErrors));

  const ownUnlock = () => {
    setProblem(null);
    unlock(secret).then(answer).catch(failed);
  };
  const adminUnlock = () => {
    setProblem(null);
    unlockByAdmin(adminName, adminPassword).then(answer).catch(failed);
  };
  const switchUser = () => {
    logout().then(onLoggedOut).catch(failed);
  };
  const onEnter = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      ownUnlock();
    }
  };

  return (
    <section
      aria-labelledby="lock-title"
      className="mx-auto flex w-full max-w-md flex-col gap-4 rounded-ui border border-border bg-panel p-6"
    >
      <h2 id="lock-title" className="flex items-center gap-2 text-[28px] font-semibold">
        <Lock aria-hidden="true" size={28} />
        {t("lock.title")}
      </h2>
      <p className="text-text-muted">{t("lock.intro", { name })}</p>
      <Field
        label={t("lock.secret.label")}
        tip={t("lock.secret.tip", { name })}
        value={secret}
        onChange={setSecret}
        onKeyDown={onEnter}
        secret
      />
      <div className="flex justify-end">
        <ActionButton
          colour="green"
          icon={Unlock}
          label={t("lock.unlock.label")}
          tip={t("lock.unlock.tip")}
          onClick={ownUnlock}
        />
      </div>
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      <div className="flex flex-col gap-3 border-t border-border pt-3">
        <h3 className="text-xl font-semibold">{t("lock.adminTitle")}</h3>
        <Field
          label={t("lock.adminUsername.label")}
          tip={t("lock.adminUsername.tip")}
          value={adminName}
          onChange={setAdminName}
        />
        <Field
          label={t("lock.adminPassword.label")}
          tip={t("lock.adminPassword.tip")}
          value={adminPassword}
          onChange={setAdminPassword}
          secret
        />
        <div className="flex justify-between">
          <ActionButton
            colour="grey"
            icon={LogOut}
            label={t("lock.switchUser.label")}
            tip={t("lock.switchUser.tip", { name })}
            onClick={switchUser}
          />
          <ActionButton
            colour="amber"
            icon={ShieldCheck}
            label={t("lock.adminUnlock.label")}
            tip={t("lock.adminUnlock.tip", { name })}
            onClick={adminUnlock}
          />
        </div>
      </div>
    </section>
  );
}
