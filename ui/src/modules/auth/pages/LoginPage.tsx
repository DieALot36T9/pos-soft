import { ArrowLeft, KeyRound, LogIn, UserRound } from "lucide-react";
import { useEffect, useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

import type { SessionView } from "../../../generated/SessionView";
import type { UserTile } from "../../../generated/UserTile";
import { errorMessage } from "../../../shared/bridge";
import { secondsUntil } from "../../../shared/format/time";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { Tip } from "../../../shared/ui/Tip";
import { login, loginTiles } from "../api";
import { RecoveryForm } from "../components/RecoveryForm";

// Which box the tile asks for: a PIN, a password, or either (USR-01).
function secretLabelKey(tile: UserTile) {
  if (tile.has_pin && tile.has_password) {
    return "either" as const;
  }
  return tile.has_pin ? ("pin" as const) : ("password" as const);
}

// The login screen: a tile for each active user, then their PIN or password (USR-01).
// Rust checks the secret and counts wrong tries (USR-02).
export function LoginPage({ onLoggedIn }: { onLoggedIn: (session: SessionView) => void }) {
  const { t } = useTranslation("auth");
  const { t: tErrors } = useTranslation("errors");
  const [tiles, setTiles] = useState<UserTile[] | null>(null);
  const [chosen, setChosen] = useState<UserTile | null>(null);
  const [secret, setSecret] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [recovering, setRecovering] = useState(false);

  useEffect(() => {
    loginTiles()
      .then(setTiles)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  }, [tErrors]);

  const choose = (tile: UserTile | null) => {
    setChosen(tile);
    setSecret("");
    setProblem(null);
  };

  const submit = () => {
    if (!chosen) {
      return;
    }
    setProblem(null);
    login(chosen.id, secret)
      .then((result) => {
        setSecret("");
        if (result.kind === "logged_in") {
          onLoggedIn(result.session);
        } else if (result.kind === "wrong_secret") {
          setProblem(t("login.wrong", { left: result.attempts_left }));
        } else {
          setProblem(t("login.locked", { seconds: secondsUntil(result.until_ms) }));
        }
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  };
  const onEnter = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      submit();
    }
  };

  if (recovering) {
    return <RecoveryForm onDone={() => setRecovering(false)} />;
  }

  return (
    <section
      aria-labelledby="login-title"
      className="mx-auto flex w-full max-w-2xl flex-col gap-4 rounded-ui border border-border bg-panel p-6"
    >
      <h2 id="login-title" className="text-[28px] font-semibold">
        {chosen ? t("login.hello", { name: chosen.display_name }) : t("login.title")}
      </h2>
      {chosen === null ? (
        <>
          <p className="text-text-muted">{t("login.intro")}</p>
          {tiles !== null && tiles.length === 0 ? (
            <Alert kind="warning">{t("login.noUsers")}</Alert>
          ) : null}
          <ul className="grid grid-cols-2 gap-3 sm:grid-cols-3">
            {(tiles ?? []).map((tile) => (
              <li key={tile.id}>
                <Tip tip={t("login.tileTip", { name: tile.display_name })}>
                  <button
                    type="button"
                    onClick={() => choose(tile)}
                    className="flex min-h-24 w-full flex-col items-center justify-center gap-2 rounded-ui border border-border bg-bg p-3 text-lg font-semibold"
                  >
                    <UserRound aria-hidden="true" size={28} />
                    {tile.display_name}
                  </button>
                </Tip>
              </li>
            ))}
          </ul>
        </>
      ) : (
        <>
          <Field
            label={t(`login.${secretLabelKey(chosen)}.label`)}
            tip={t(`login.${secretLabelKey(chosen)}.tip`)}
            value={secret}
            onChange={setSecret}
            onKeyDown={onEnter}
            secret
          />
          <div className="flex justify-between">
            <ActionButton
              colour="grey"
              icon={ArrowLeft}
              label={t("login.back.label")}
              tip={t("login.back.tip")}
              onClick={() => choose(null)}
            />
            <ActionButton
              colour="green"
              icon={LogIn}
              label={t("login.button.label")}
              tip={t("login.button.tip")}
              onClick={submit}
            />
          </div>
        </>
      )}
      {problem ? <Alert kind="problem">{problem}</Alert> : null}
      <div className="border-t border-border pt-3">
        <ActionButton
          colour="grey"
          icon={KeyRound}
          label={t("login.forgot.label")}
          tip={t("login.forgot.tip")}
          onClick={() => setRecovering(true)}
        />
      </div>
    </section>
  );
}
