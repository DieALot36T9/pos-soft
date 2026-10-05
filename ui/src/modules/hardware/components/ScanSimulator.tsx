import { ScanBarcode, ScanLine, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useTranslation } from "react-i18next";

import { shortcuts } from "../../../shared/keyboard/shortcuts";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Field } from "../../../shared/ui/Field";
import { IconButton } from "../../../shared/ui/IconButton";
import { Tip } from "../../../shared/ui/Tip";
import { sampleCodes } from "./sampleCodes";
import { typeLikeScanner } from "./typeLikeScanner";

// Dev only: "scans" a code into the text box that last had the cursor, at scanner
// speed (rule HW-10). Opened with Ctrl+Shift+S or its button.
export function ScanSimulator() {
  const { t } = useTranslation("hardware");
  const [open, setOpen] = useState(false);
  const [typed, setTyped] = useState("");
  const [problem, setProblem] = useState<string | null>(null);
  const panel = useRef<HTMLDivElement>(null);
  const target = useRef<HTMLInputElement | null>(null);

  useHotkeys(shortcuts.scanSimulator.keys, () => setOpen((wasOpen) => !wasOpen), {
    enableOnFormTags: true,
    preventDefault: true,
  });

  // Remembers the last text box outside this panel that had the cursor.
  useEffect(() => {
    const remember = (event: FocusEvent) => {
      const element = event.target;
      if (
        element instanceof HTMLInputElement &&
        element.type === "text" &&
        !panel.current?.contains(element)
      ) {
        target.current = element;
      }
    };
    document.addEventListener("focusin", remember);
    return () => document.removeEventListener("focusin", remember);
  }, []);

  const scan = (code: string) => {
    const input = target.current;
    if (!input || !input.isConnected) {
      setProblem(t("scanner.noTarget"));
      return;
    }
    setProblem(null);
    void typeLikeScanner(input, code);
  };

  if (!open) {
    return (
      <div className="fixed right-4 bottom-4">
        <ActionButton
          colour="grey"
          icon={ScanBarcode}
          label={t("scanner.openButton.label")}
          tip={t("scanner.openButton.tip")}
          shortcut={shortcuts.scanSimulator.label}
          onClick={() => setOpen(true)}
        />
      </div>
    );
  }

  return (
    <div
      ref={panel}
      role="region"
      aria-label={t("scanner.heading")}
      className="fixed right-4 bottom-4 w-96 rounded-ui border border-border bg-panel p-4"
    >
      <div className="mb-2 flex items-center justify-between">
        <h3 className="text-xl font-semibold">{t("scanner.heading")}</h3>
        <IconButton
          icon={X}
          tip={t("scanner.openButton.tip")}
          shortcut={shortcuts.scanSimulator.label}
          onClick={() => setOpen(false)}
        />
      </div>
      <p className="mb-3 text-text-muted">{t("scanner.intro")}</p>
      <ul className="mb-3 flex flex-col gap-1">
        {sampleCodes.map(({ code, kind }) => (
          <li key={code}>
            <Tip tip={t("scanner.codeButtonTip", { code })}>
              <button
                type="button"
                onClick={() => scan(code)}
                className="flex min-h-10 w-full items-center gap-2 rounded-ui border border-blue bg-blue-soft px-3 text-left text-blue"
              >
                <ScanLine aria-hidden="true" size={18} />
                <span className="font-mono font-semibold">{code}</span>
                <span className="text-sm">{t(`scanner.samples.${kind}`)}</span>
              </button>
            </Tip>
          </li>
        ))}
      </ul>
      <div className="flex items-end gap-2">
        <Field
          label={t("scanner.typed.label")}
          tip={t("scanner.typed.tip")}
          value={typed}
          onChange={setTyped}
        />
        <ActionButton
          colour="blue"
          icon={ScanLine}
          label={t("scanner.scanButton.label")}
          tip={t("scanner.scanButton.tip")}
          onClick={() => scan(typed)}
        />
      </div>
      {problem ? (
        <p
          role="alert"
          className="mt-3 rounded-ui border border-amber bg-amber-soft p-3 text-amber"
        >
          {problem}
        </p>
      ) : null}
    </div>
  );
}
