import { useState } from "react";
import { useTranslation } from "react-i18next";

import type { DrawerPin } from "../../../generated/DrawerPin";
import type { PrinterConnection } from "../../../generated/PrinterConnection";
import { useSave } from "../../../shared/bridge/useSave";
import { Field } from "../../../shared/ui/Field";
import { SelectField } from "../../../shared/ui/SelectField";
import { saveSettings } from "../api";
import { SectionFrame } from "./SectionFrame";
import type { SectionProps } from "./types";

const printerKinds = ["preview", "windows_spooler", "tcp", "serial", "cups"] as const;
type PrinterKind = (typeof printerKinds)[number];
// The scale profiles built into the hardware module (HW-05).
const scaleProfiles = ["generic_continuous", "cas", "toledo"] as const;
type ScaleChoice = "none" | (typeof scaleProfiles)[number];

// The receipt printer, the drawer pin and the scale (SET-10, HW-02, HW-04, HW-05).
// Rust checks every connection detail when saving.
export function DevicesSection({ saved, onSaved }: SectionProps) {
  const { t } = useTranslation("settings");
  const printer = saved.hardware.receipt_printer;
  const [kind, setKind] = useState<PrinterKind>(
    printer === null || printer.kind === "file" ? "preview" : printer.kind,
  );
  // One text box per detail; only the ones the chosen kind needs are shown.
  const [name, setName] = useState(
    printer?.kind === "windows_spooler"
      ? printer.printer_name
      : printer?.kind === "cups"
        ? printer.queue
        : "",
  );
  const [host, setHost] = useState(printer?.kind === "tcp" ? printer.host : "");
  const [tcpPort, setTcpPort] = useState(printer?.kind === "tcp" ? String(printer.port) : "9100");
  const [serialPort, setSerialPort] = useState(printer?.kind === "serial" ? printer.port : "");
  const [baud, setBaud] = useState(printer?.kind === "serial" ? String(printer.baud) : "9600");
  const [pin, setPin] = useState<DrawerPin>(saved.hardware.drawer_pin);
  const scale = saved.hardware.scale;
  const [scaleChoice, setScaleChoice] = useState<ScaleChoice>(
    scale === null
      ? "none"
      : (scaleProfiles.find((each) => each === scale.profile) ?? "generic_continuous"),
  );
  const [scalePort, setScalePort] = useState(scale?.port ?? "");
  const [scaleBaud, setScaleBaud] = useState(String(scale?.baud ?? 9600));
  const save = useSave();

  const connection = (): PrinterConnection => {
    switch (kind) {
      case "windows_spooler":
        return { kind, printer_name: name.trim() };
      case "tcp":
        return { kind, host: host.trim(), port: Number(tcpPort) || 0 };
      case "serial":
        return { kind, port: serialPort.trim(), baud: Number(baud) || 0 };
      case "cups":
        return { kind, queue: name.trim() };
      case "preview":
        return { kind };
    }
  };
  const submit = () =>
    save.run(
      () =>
        saveSettings({
          section: "hardware",
          value: {
            ...saved.hardware,
            receipt_printer: connection(),
            drawer_pin: pin,
            scale:
              scaleChoice === "none"
                ? null
                : {
                    port: scalePort.trim(),
                    baud: Number(scaleBaud) || 0,
                    profile: scaleChoice,
                  },
          },
        }),
      onSaved,
    );

  return (
    <SectionFrame
      title={t("sections.devices")}
      busy={save.busy}
      done={save.done}
      problem={save.problem}
      onSave={submit}
    >
      <SelectField
        label={t("devices.printer.label")}
        tip={t("devices.printer.tip")}
        value={kind}
        options={printerKinds.map((value) => ({ value, label: t(`devices.printer.${value}`) }))}
        onChange={setKind}
        error={save.errorFor("receipt_printer")}
      />
      {kind === "windows_spooler" || kind === "cups" ? (
        <Field
          label={t(kind === "cups" ? "devices.queue.label" : "devices.name.label")}
          tip={t(kind === "cups" ? "devices.queue.tip" : "devices.name.tip")}
          value={name}
          onChange={setName}
        />
      ) : null}
      {kind === "tcp" ? (
        <div className="grid grid-cols-2 gap-4">
          <Field
            label={t("devices.host.label")}
            tip={t("devices.host.tip")}
            value={host}
            onChange={setHost}
          />
          <Field
            label={t("devices.tcpPort.label")}
            tip={t("devices.tcpPort.tip")}
            value={tcpPort}
            onChange={setTcpPort}
            width="short"
          />
        </div>
      ) : null}
      {kind === "serial" ? (
        <div className="grid grid-cols-2 gap-4">
          <Field
            label={t("devices.serialPort.label")}
            tip={t("devices.serialPort.tip")}
            value={serialPort}
            onChange={setSerialPort}
          />
          <Field
            label={t("devices.baud.label")}
            tip={t("devices.baud.tip")}
            value={baud}
            onChange={setBaud}
            width="short"
          />
        </div>
      ) : null}
      <SelectField
        label={t("devices.drawer.label")}
        tip={t("devices.drawer.tip")}
        value={pin}
        options={(["pin2", "pin5"] as const).map((value) => ({
          value,
          label: t(`devices.drawer.${value}`),
        }))}
        onChange={setPin}
      />
      <SelectField
        label={t("devices.scale.label")}
        tip={t("devices.scale.tip")}
        value={scaleChoice}
        options={(["none", ...scaleProfiles] as const).map((value) => ({
          value,
          label: t(`devices.scale.${value}`),
        }))}
        onChange={setScaleChoice}
        error={save.errorFor("scale")}
      />
      {scaleChoice !== "none" ? (
        <div className="grid grid-cols-2 gap-4">
          <Field
            label={t("devices.scalePort.label")}
            tip={t("devices.scalePort.tip")}
            value={scalePort}
            onChange={setScalePort}
          />
          <Field
            label={t("devices.scaleBaud.label")}
            tip={t("devices.scaleBaud.tip")}
            value={scaleBaud}
            onChange={setScaleBaud}
            width="short"
          />
        </div>
      ) : null}
      <p className="text-text-muted">{t("devices.testNote")}</p>
    </SectionFrame>
  );
}
