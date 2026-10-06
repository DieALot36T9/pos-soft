import { Save } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { SecretKind } from "../../../generated/SecretKind";
import { errorMessage } from "../../../shared/bridge";
import { errorField } from "../../../shared/bridge/fieldError";
import { secondsUntil } from "../../../shared/format/time";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { changeOwnSecret } from "../api";

// "My PIN and password" (USR-07): change your own PIN or password after typing the
// current one. Rust checks the current one and the new one's format.
export function MyAccountPage() {
  const { t } = useTranslation("auth");
  const { t: tErrors } = useTranslation("errors");
  const [kind, setKind] = useState<SecretKind>("pin");
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [again, setAgain] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const [newProblem, setNewProblem] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  const submit = () => {
    setProblem(null);
    setNewProblem(null);
    setSaved(false);
    if (next !== again) {
      setNewProblem(t("account.differ"));
      return;
    }
    changeOwnSecret(kind, current, next)
      .then((result) => {
        setCurrent("");
        if (result.kind === "accepted") {
          setNext("");
          setAgain("");
          setSaved(true);
        } else if (result.kind === "wrong_secret") {
          setProblem(t("account.wrong", { left: result.attempts_left }));
        } else if (result.kind === "locked") {
          setProblem(t("account.locked", { seconds: secondsUntil(result.until_ms) }));
        }
      })
      .catch((error: unknown) => {
        const message = errorMessage(error, tErrors);
        if (errorField(error) === "pin" || errorField(error) === "password") {
          setNewProblem(message);
        } else {
          setProblem(message);
        }
      });
  };

  return (
    <section aria-labelledby="account-heading" className="flex max-w-xl flex-col gap-4">
      <div>
        <h2 id="account-heading" className="text-[28px] font-semibold">
          {t("account.title")}
        </h2>
        <p className="text-text-muted">{t("account.intro")}</p>
      </div>
      <div className="flex flex-col gap-3 rounded-ui border border-border bg-panel p-4">
        <SelectField
          label={t("account.kind.label")}
          tip={t("account.kind.tip")}
          value={kind}
          options={[
            { value: "pin", label: t("account.kinds.pin") },
            { value: "password", label: t("account.kinds.password") },
          ]}
          onChange={setKind}
        />
        <Field
          label={t("account.current.label")}
          tip={t("account.current.tip")}
          value={current}
          onChange={setCurrent}
          secret
        />
        <Field
          label={kind === "pin" ? t("account.newPin.label") : t("account.newPassword.label")}
          tip={kind === "pin" ? t("account.newPin.tip") : t("account.newPassword.tip")}
          value={next}
          onChange={setNext}
          error={newProblem}
          secret
        />
        <Field
          label={t("account.again.label")}
          tip={t("account.again.tip")}
          value={again}
          onChange={setAgain}
          secret
        />
        {problem ? <Alert kind="problem">{problem}</Alert> : null}
        {saved ? <Alert kind="done">{t("account.saved")}</Alert> : null}
        <div className="flex justify-end">
          <ActionButton
            colour="green"
            icon={Save}
            label={t("account.save.label")}
            tip={t("account.save.tip")}
            onClick={submit}
          />
        </div>
      </div>
    </section>
  );
}
