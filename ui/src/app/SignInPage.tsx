import { LogIn } from "lucide-react";
import { useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

import type { SessionView } from "../generated/SessionView";
import { useSave } from "../shared/bridge/useSave";
import { ActionButton } from "../shared/ui/ActionButton";
import { Alert } from "../shared/ui/Alert";
import { Field } from "../shared/ui/Field";
import { signIn } from "./api";

// Username and password, checked by Rust. C-M3 replaces this with the login tiles and PINs.
export function SignInPage({ onSignedIn }: { onSignedIn: (session: SessionView) => void }) {
  const { t } = useTranslation("app");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [refusal, setRefusal] = useState<string | null>(null);
  const save = useSave();

  const submit = () => {
    setRefusal(null);
    save.run(
      () => signIn(username, password),
      (result) => {
        if (result.kind === "signed_in") {
          onSignedIn(result.session);
        } else if (result.kind === "locked") {
          setRefusal(t("signIn.locked"));
        } else if (result.attempts_left !== null) {
          setRefusal(t("signIn.refusedLeft", { left: result.attempts_left }));
        } else {
          setRefusal(t("signIn.refused"));
        }
      },
    );
  };
  const onEnter = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === "Enter") {
      submit();
    }
  };

  return (
    <section
      aria-labelledby="sign-in-title"
      className="mx-auto flex w-full max-w-md flex-col gap-4 rounded-ui border border-border bg-panel p-6"
    >
      <h2 id="sign-in-title" className="text-[28px] font-semibold">
        {t("signIn.title")}
      </h2>
      <p className="text-text-muted">{t("signIn.intro")}</p>
      <Field
        label={t("signIn.username.label")}
        tip={t("signIn.username.tip")}
        value={username}
        onChange={setUsername}
        onKeyDown={onEnter}
      />
      <Field
        label={t("signIn.password.label")}
        tip={t("signIn.password.tip")}
        value={password}
        onChange={setPassword}
        onKeyDown={onEnter}
        secret
      />
      {refusal ? <Alert kind="problem">{refusal}</Alert> : null}
      {save.problem ? <Alert kind="problem">{save.problem}</Alert> : null}
      <div className="flex justify-end">
        <ActionButton
          colour="green"
          icon={LogIn}
          label={t("signIn.button.label")}
          tip={t("signIn.button.tip")}
          onClick={submit}
        />
      </div>
    </section>
  );
}
