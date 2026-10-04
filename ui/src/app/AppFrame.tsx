import { Info } from "lucide-react";
import { useState } from "react";

import type { AppInfo } from "../generated/AppInfo";
import { errorMessage } from "../shared/bridge";
import { commonText, errorText, useText } from "../shared/text";
import { ActionButton } from "../shared/ui/ActionButton";
import { getAppInfo } from "./api";
import { appText } from "./text";

// The empty app window: a flat top bar and one panel. Screens are added in later milestones.
export function AppFrame() {
  const text = useText(appText);
  const common = useText(commonText);
  const errors = useText(errorText);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [problem, setProblem] = useState<string | null>(null);

  const checkApp = () => {
    getAppInfo()
      .then((answer) => {
        setInfo(answer);
        setProblem(null);
      })
      .catch((error: unknown) => setProblem(errorMessage(error, errors)));
  };

  return (
    <div className="flex min-h-screen flex-col bg-bg text-text">
      <header className="border-b border-border bg-panel px-6 py-3">
        <h1 className="text-xl font-semibold">{common.appName}</h1>
      </header>
      <main className="flex flex-1 items-start justify-center p-6">
        <section className="w-full max-w-xl rounded-ui border border-border bg-panel p-6">
          <h2 className="mb-2 text-[28px] font-semibold">{text.title}</h2>
          <p className="mb-6 text-text-muted">{text.intro}</p>
          <ActionButton
            colour="blue"
            icon={Info}
            label={text.checkButton.label}
            tip={text.checkButton.tip}
            onClick={checkApp}
          />
          {info ? (
            <p className="mt-4">
              {text.versionLabel}
              <span className="ml-1 font-semibold tabular-nums">{info.version}</span>
            </p>
          ) : null}
          {problem ? (
            <p role="alert" className="mt-4 rounded-ui border border-red bg-red-soft p-3 text-red">
              {problem}
            </p>
          ) : null}
        </section>
      </main>
    </div>
  );
}
