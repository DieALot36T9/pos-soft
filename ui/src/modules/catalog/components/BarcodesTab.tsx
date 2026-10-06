import { Plus, ScanBarcode, Star, Trash2 } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";

import { ActionButton } from "../../../shared/ui/ActionButton";
import { Badge } from "../../../shared/ui/Badge";
import { Field } from "../../../shared/ui/Field";
import { IconButton } from "../../../shared/ui/IconButton";
import type { DraftProps } from "./draft";

type BarcodesTabProps = DraftProps & {
  // Null until the product is saved: a shop barcode is added straight to a saved product.
  onGenerate: (() => void) | null;
  isParent: boolean;
};

// The product's barcodes (CAT-04) and "Make shop barcode" (CAT-11).
export function BarcodesTab({ draft, change, errorFor, onGenerate, isParent }: BarcodesTabProps) {
  const { t } = useTranslation("catalog");
  const [typed, setTyped] = useState("");

  const add = () => {
    const code = typed.trim();
    if (code !== "" && !draft.barcodes.includes(code)) {
      change({ barcodes: [...draft.barcodes, code] });
    }
    setTyped("");
  };

  return (
    <div className="flex flex-col gap-3">
      <p className="text-text-muted">{t("barcodes.intro")}</p>
      {draft.barcodes.length === 0 ? <p>{t("barcodes.empty")}</p> : null}
      <ul className="flex flex-col gap-2">
        {draft.barcodes.map((code, index) => (
          <li key={code} className="flex items-center gap-3">
            <span className="min-w-48 font-semibold tabular-nums">{code}</span>
            {index === 0 ? (
              <Badge
                colour="blue"
                icon={Star}
                word={t("barcodes.main")}
                tip={t("barcodes.mainTip")}
              />
            ) : null}
            <IconButton
              icon={Trash2}
              colour="red"
              tip={t("barcodes.remove.tip")}
              onClick={() => change({ barcodes: draft.barcodes.filter((each) => each !== code) })}
            />
          </li>
        ))}
      </ul>
      {errorFor("barcodes") ? <p className="text-red">{errorFor("barcodes")}</p> : null}
      <div className="flex items-end gap-3">
        <Field
          label={t("barcodes.new.label")}
          tip={t("barcodes.new.tip")}
          value={typed}
          onChange={setTyped}
          onKeyDown={(event) => {
            // A scanner ends with Enter: add the code instead of submitting anything.
            if (event.key === "Enter") {
              event.preventDefault();
              add();
            }
          }}
        />
        <ActionButton
          colour="blue"
          icon={Plus}
          label={t("barcodes.add.label")}
          tip={t("barcodes.add.tip")}
          disabledReason={isParent ? t("barcodes.parentHasNone") : undefined}
          onClick={add}
        />
        <ActionButton
          colour="blue"
          icon={ScanBarcode}
          label={t("barcodes.generate.label")}
          tip={t("barcodes.generate.tip")}
          disabledReason={
            isParent ? t("barcodes.parentHasNone") : onGenerate ? undefined : t("editor.saveFirst")
          }
          onClick={() => onGenerate?.()}
        />
      </div>
    </div>
  );
}
