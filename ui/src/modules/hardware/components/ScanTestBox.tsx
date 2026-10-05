import { useRef, useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";

import { Field } from "../../../shared/ui/Field";

// A box to try the scanner (or the scan simulator): Enter shows what arrived and how
// long the typing took. Checkout's scan box (C-M6) works the same way (rule HW-01).
export function ScanTestBox() {
  const { t } = useTranslation("hardware");
  const [value, setValue] = useState("");
  const [received, setReceived] = useState<{ code: string; ms: number } | null>(null);
  const firstKeyAt = useRef<number | null>(null);

  const change = (text: string) => {
    if (firstKeyAt.current === null) {
      firstKeyAt.current = performance.now();
    }
    setValue(text);
  };

  const keyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key !== "Enter") {
      return;
    }
    const started = firstKeyAt.current ?? performance.now();
    setReceived({ code: event.currentTarget.value, ms: Math.round(performance.now() - started) });
    firstKeyAt.current = null;
    setValue("");
  };

  return (
    <section className="rounded-ui border border-border bg-panel p-4">
      <h3 className="mb-3 text-xl font-semibold">{t("scanBox.heading")}</h3>
      <Field
        label={t("scanBox.field.label")}
        tip={t("scanBox.field.tip")}
        value={value}
        onChange={change}
        onKeyDown={keyDown}
      />
      {received ? (
        <p role="status" className="mt-3 font-mono">
          {t("scanBox.received", received)}
        </p>
      ) : null}
    </section>
  );
}
