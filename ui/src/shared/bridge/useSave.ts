import { useState } from "react";
import { useTranslation } from "react-i18next";

import { errorMessage } from "./errors";
import { errorField } from "./fieldError";

// The usual way a form saves: busy while Rust works, then the problem (if any)
// and which field it belongs to, or "done".
export function useSave() {
  const { t } = useTranslation("errors");
  const [busy, setBusy] = useState(false);
  const [problem, setProblem] = useState<string | null>(null);
  const [badField, setBadField] = useState<string | null>(null);
  const [done, setDone] = useState(false);

  // Runs the save. `onDone` gets Rust's answer when it worked.
  const run = <T>(work: () => Promise<T>, onDone?: (answer: T) => void) => {
    setBusy(true);
    setProblem(null);
    setBadField(null);
    setDone(false);
    work()
      .then((answer) => {
        setDone(true);
        onDone?.(answer);
      })
      .catch((error: unknown) => {
        setProblem(errorMessage(error, t));
        setBadField(errorField(error));
      })
      .finally(() => setBusy(false));
  };

  // The message to show under a field, when Rust refused that field.
  const errorFor = (field: string) => (badField === field ? problem : null);

  return { busy, problem, done, run, errorFor };
}
