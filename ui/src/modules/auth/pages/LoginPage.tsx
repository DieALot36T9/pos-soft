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
import { KitNumpad } from "../../../shared/ui/PosKit";
import { Tip } from "../../../shared/ui/Tip";
import { login, loginTiles } from "../api";
import { RecoveryForm } from "../components/RecoveryForm";

function secretLabelKey(tile: UserTile) {
  if (tile.has_pin && tile.has_password) {
    return "either" as const;
  }
  return tile.has_pin ? ("pin" as const) : ("password" as const);
}

function getInitials(name: string) {
  return name
    .split(/\s+/)
    .map((w) => w[0])
    .filter(Boolean)
    .join("")
    .slice(0, 2)
    .toUpperCase();
}

function getRoleBadge(tile: UserTile) {
  if (tile.has_password && !tile.has_pin) return "Admin / Owner";
  if (tile.has_pin && !tile.has_password) return "Cashier";
  return "Staff";
}

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

  const selectStaffLabel = "Select Staff User";
  const switchUserLabel = "Switch user";
  const selectYourNameTitle = "Select your name";
  const selectYourNameDesc =
    "Select your name on the left, then enter your PIN or password here to sign in.";

  return (
    <section
      aria-labelledby="login-title"
      className="mx-auto flex w-full max-w-4xl flex-col gap-6 rounded-[6px] border border-border bg-panel p-6"
    >
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-border pb-4">
        <div>
          <h2 id="login-title" className="text-2xl font-bold tracking-tight text-text">
            {chosen ? t("login.hello", { name: chosen.display_name }) : t("login.title")}
          </h2>
          <p className="text-sm text-text-muted mt-0.5">
            {chosen ? "Enter your PIN or password on the right to sign in." : t("login.intro")}
          </p>
        </div>
        {chosen && (
          <div className="flex items-center gap-2">
            <span className="px-2.5 py-0.5 rounded-[6px] text-xs font-semibold bg-blue-soft text-blue border border-blue/30">
              {getRoleBadge(chosen)}
            </span>
          </div>
        )}
      </div>

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 items-start">
        {/* Left Column: User Tiles */}
        <div className="flex flex-col gap-3">
          <div className="flex items-center justify-between">
            <span className="text-xs font-bold uppercase tracking-wider text-text-muted">
              {selectStaffLabel}
            </span>
            {chosen && (
              <button
                type="button"
                onClick={() => choose(null)}
                className="text-xs font-semibold text-blue hover:underline cursor-pointer"
              >
                <span>{switchUserLabel}</span>
              </button>
            )}
          </div>

          {tiles !== null && tiles.length === 0 ? (
            <Alert kind="warning">{t("login.noUsers")}</Alert>
          ) : null}

          <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-1 gap-2.5 max-h-[420px] overflow-y-auto pr-1">
            {(tiles ?? []).map((tile) => {
              const isSelected = chosen?.id === tile.id;
              return (
                <Tip key={tile.id} tip={t("login.tileTip", { name: tile.display_name })}>
                  <button
                    type="button"
                    onClick={() => choose(tile)}
                    className={`flex items-center gap-3 p-3 rounded-[6px] border text-left cursor-pointer ${
                      isSelected
                        ? "border-blue bg-blue-soft text-blue font-bold"
                        : "border-border bg-bg hover:bg-grey-soft text-text"
                    }`}
                  >
                    <div
                      className={`w-10 h-10 rounded-full flex items-center justify-center font-bold text-sm shrink-0 border ${
                        isSelected
                          ? "bg-blue text-white border-blue"
                          : "bg-panel text-text border-border"
                      }`}
                    >
                      {getInitials(tile.display_name)}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-semibold truncate">{tile.display_name}</div>
                      <div className="text-xs text-text-muted flex items-center gap-1.5 mt-0.5">
                        <span className="px-1.5 py-0.2 rounded bg-panel border border-border text-[10px] font-medium">
                          {getRoleBadge(tile)}
                        </span>
                        <span>{tile.has_pin ? "PIN" : "Password"}</span>
                      </div>
                    </div>
                  </button>
                </Tip>
              );
            })}
          </div>
        </div>

        {/* Right Column: Keypad & Input */}
        <div className="flex flex-col gap-4 bg-bg border border-border rounded-[6px] p-5">
          {chosen === null ? (
            <div className="py-12 px-4 flex flex-col items-center justify-center text-center text-text-muted">
              <UserRound className="w-12 h-12 mb-3 text-text-muted/60" />
              <p className="text-base font-semibold text-text">{selectYourNameTitle}</p>
              <p className="text-xs text-text-muted mt-1 max-w-xs">{selectYourNameDesc}</p>
            </div>
          ) : (
            <div className="flex flex-col gap-4">
              <Field
                label={t(`login.${secretLabelKey(chosen)}.label`)}
                tip={t(`login.${secretLabelKey(chosen)}.tip`)}
                value={secret}
                onChange={setSecret}
                onKeyDown={onEnter}
                secret
              />

              {/* Integrated touch numpad for PIN entry */}
              {chosen.has_pin && (
                <div className="pt-1">
                  <KitNumpad
                    value={secret}
                    onChange={setSecret}
                    onEnter={submit}
                    allowDecimal={false}
                  />
                </div>
              )}

              <div className="flex justify-between items-center pt-2">
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
            </div>
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
        </div>
      </div>
    </section>
  );
}
