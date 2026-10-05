import { CheckCircle, PauseCircle, Save, Search } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import type { Reading } from "../../../generated/Reading";
import type { WeightUnit } from "../../../generated/WeightUnit";
import { errorMessage } from "../../../shared/bridge";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Badge } from "../../../shared/ui/Badge";
import { CheckBox } from "../../../shared/ui/CheckBox";
import { Field } from "../../../shared/ui/Field";
import { Tip } from "../../../shared/ui/Tip";
import { getFakeScale, readFakeScale, setFakeScale } from "../api";

const units: WeightUnit[] = ["kg", "g", "lb"];

// True when Rust refused the value typed into one field.
function isFieldError(error: unknown, field: string): boolean {
  return typeof error === "object" && error !== null && "field" in error && error.field === field;
}

// Dev only: set the fake scale's weight, make it move or unplug it, and read it
// the way checkout will (rule HW-10).
export function FakeScalePanel() {
  const { t } = useTranslation("hardware");
  const { t: tErrors } = useTranslation("errors");
  const [weight, setWeight] = useState("0.000");
  const [unit, setUnit] = useState<WeightUnit>("kg");
  const [stable, setStable] = useState(true);
  const [connected, setConnected] = useState(true);
  const [weightError, setWeightError] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [reading, setReading] = useState<Reading | null>(null);
  const [readingBusy, setReadingBusy] = useState(false);

  useEffect(() => {
    getFakeScale()
      .then((view) => {
        setWeight(view.weight);
        setUnit(view.unit);
        setStable(view.stable);
        setConnected(view.connected);
      })
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)));
  }, [tErrors]);

  const save = () => {
    setWeightError(null);
    setProblem(null);
    setFakeScale({ weight, unit, stable, connected })
      .then((view) => setWeight(view.weight))
      .catch((error: unknown) => {
        const message = errorMessage(error, tErrors);
        if (isFieldError(error, "weight")) {
          setWeightError(message);
        } else {
          setProblem(message);
        }
      });
  };

  const read = () => {
    setProblem(null);
    setReading(null);
    setReadingBusy(true);
    readFakeScale()
      .then(setReading)
      .catch((error: unknown) => setProblem(errorMessage(error, tErrors)))
      .finally(() => setReadingBusy(false));
  };

  return (
    <section className="rounded-ui border border-border bg-panel p-4">
      <h3 className="mb-1 text-xl font-semibold">{t("scale.heading")}</h3>
      <p className="mb-3 text-text-muted">{t("scale.intro")}</p>
      <div className="mb-3 flex flex-wrap items-end gap-4">
        <Field
          label={t("scale.weight.label")}
          tip={t("scale.weight.tip")}
          value={weight}
          onChange={setWeight}
          error={weightError}
          width="short"
        />
        <div className="flex flex-col gap-1">
          <label htmlFor="fake-scale-unit" className="font-semibold">
            {t("scale.unit.label")}
          </label>
          <Tip tip={t("scale.unit.tip")}>
            <select
              id="fake-scale-unit"
              value={unit}
              onChange={(event) => setUnit(event.target.value as WeightUnit)}
              className="min-h-10 rounded-ui border border-border bg-panel px-3"
            >
              {units.map((choice) => (
                <option key={choice} value={choice}>
                  {t(`scale.unit.${choice}`)}
                </option>
              ))}
            </select>
          </Tip>
        </div>
      </div>
      <div className="mb-4 flex flex-col gap-2">
        <CheckBox
          label={t("scale.stable.label")}
          tip={t("scale.stable.tip")}
          checked={stable}
          onChange={setStable}
        />
        <CheckBox
          label={t("scale.connected.label")}
          tip={t("scale.connected.tip")}
          checked={connected}
          onChange={setConnected}
        />
      </div>
      <div className="flex flex-wrap gap-2">
        <ActionButton
          colour="green"
          icon={Save}
          label={t("scale.saveButton.label")}
          tip={t("scale.saveButton.tip")}
          onClick={save}
        />
        <ActionButton
          colour="blue"
          icon={Search}
          label={t("scale.readButton.label")}
          tip={t("scale.readButton.tip")}
          disabledReason={readingBusy ? t("scale.busy") : undefined}
          onClick={read}
        />
      </div>
      {reading ? (
        <div role="status" className="mt-3 flex flex-wrap items-center gap-2">
          {reading.stable ? (
            <Badge
              colour="green"
              icon={CheckCircle}
              word={t("scale.readingStable")}
              tip={t("scale.stable.tip")}
            />
          ) : (
            <Badge
              colour="amber"
              icon={PauseCircle}
              word={t("scale.readingMoving")}
              tip={t("scale.stable.tip")}
            />
          )}
          <span className="font-mono">{t("scale.readingRaw", { raw: reading.raw })}</span>
        </div>
      ) : null}
      {problem ? (
        <p role="alert" className="mt-3 rounded-ui border border-red bg-red-soft p-3 text-red">
          {problem}
        </p>
      ) : null}
    </section>
  );
}
