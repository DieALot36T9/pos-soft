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
import { KitNumpad } from "../../../shared/ui/PosKit";
import { logout, unlock, unlockByAdmin } from "../api";

type LockScreenProps = {
  session: SessionView;
  onUnlocked: () => void;
  onLoggedOut: () => void;
};

function getInitials(name: string) {
  return name
    .split(/\s+/)
    .map((w) => w[0])
    .filter(Boolean)
    .join("")
    .slice(0, 2)
    .toUpperCase();
}

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

  const terminalLockedLabel = "Terminal Locked";
  const activeUserLabel = "Active Cashier / User";

  return (
    <section
      aria-labelledby="lock-title"
      className="mx-auto flex w-full max-w-3xl flex-col gap-6 rounded-[6px] border border-border bg-panel p-6"
    >
      <div className="flex items-center justify-between border-b border-border pb-4">
        <h2
          id="lock-title"
          className="flex items-center gap-2 text-2xl font-bold tracking-tight text-text"
        >
          <Lock aria-hidden="true" className="w-6 h-6 text-amber" />
          {t("lock.title")}
        </h2>
        <span className="px-2.5 py-0.5 rounded-[6px] text-xs font-semibold bg-amber-soft text-amber border border-amber/30">
          {terminalLockedLabel}
        </span>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 items-start">
        {/* Left Column: Active User Identity Tile */}
        <div className="flex flex-col gap-4 p-5 rounded-[6px] border border-border bg-bg">
          <span className="text-xs font-bold uppercase tracking-wider text-text-muted">
            {activeUserLabel}
          </span>
          <div className="flex items-center gap-3">
            <div className="w-14 h-14 rounded-full bg-blue-soft text-blue border border-blue/30 flex items-center justify-center font-bold text-lg shrink-0">
              {getInitials(name)}
            </div>
            <div>
              <h3 className="text-lg font-bold text-text">{name}</h3>
              <p className="text-xs text-text-muted mt-0.5">{session.role_name}</p>
            </div>
          </div>

          <div className="text-xs text-text-muted leading-relaxed pt-2 border-t border-border">
            {t("lock.intro", { name })}
          </div>

          <div className="mt-auto pt-4">
            <ActionButton
              colour="grey"
              icon={LogOut}
              label={t("lock.switchUser.label")}
              tip={t("lock.switchUser.tip", { name })}
              onClick={switchUser}
            />
          </div>
        </div>

        {/* Right Column: Keypad & Admin Override */}
        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-3 p-5 rounded-[6px] border border-border bg-bg">
            <Field
              label={t("lock.secret.label")}
              tip={t("lock.secret.tip", { name })}
              value={secret}
              onChange={setSecret}
              onKeyDown={onEnter}
              secret
            />

            <KitNumpad
              value={secret}
              onChange={setSecret}
              onEnter={ownUnlock}
              allowDecimal={false}
            />

            <div className="flex justify-end pt-1">
              <ActionButton
                colour="green"
                icon={Unlock}
                label={t("lock.unlock.label")}
                tip={t("lock.unlock.tip")}
                onClick={ownUnlock}
              />
            </div>
          </div>

          {problem ? <Alert kind="problem">{problem}</Alert> : null}

          {/* Admin Unlock Accordion / Panel */}
          <div className="flex flex-col gap-3 rounded-[6px] border border-border bg-panel p-4">
            <div className="flex items-center gap-2 text-sm font-bold text-text">
              <ShieldCheck className="w-4 h-4 text-amber" />
              <span>{t("lock.adminTitle")}</span>
            </div>
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
            <div className="flex justify-end pt-1">
              <ActionButton
                colour="amber"
                icon={ShieldCheck}
                label={t("lock.adminUnlock.label")}
                tip={t("lock.adminUnlock.tip", { name })}
                onClick={adminUnlock}
              />
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
