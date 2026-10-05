import { useTranslation } from "react-i18next";

import { FakeScalePanel } from "../components/FakeScalePanel";
import { PreviewPrinterPanel } from "../components/PreviewPrinterPanel";
import { ScanSimulator } from "../components/ScanSimulator";
import { ScanTestBox } from "../components/ScanTestBox";
import { TestPrintPanel } from "../components/TestPrintPanel";
import { usePrintouts } from "../usePrintouts";

type HardwarePageProps = {
  // True in builds with the dev tools: shows the fake scale and the scan simulator.
  devTools: boolean;
};

// Working without hardware (rule HW-10): test print, the Preview printer, and in
// dev builds the fake scale, a scan test box and the scan simulator.
export function HardwarePage({ devTools }: HardwarePageProps) {
  const { t } = useTranslation("hardware");
  const { printouts, checkNow, clear } = usePrintouts();

  return (
    <section aria-labelledby="hardware-heading" className="w-full">
      <h2 id="hardware-heading" className="mb-1 text-[28px] font-semibold">
        {t("page.title")}
      </h2>
      <p className="mb-4 text-text-muted">{t("page.intro")}</p>
      <div className="grid grid-cols-1 items-start gap-4 lg:grid-cols-2">
        <div className="flex flex-col gap-4">
          <TestPrintPanel onPrinted={checkNow} />
          {devTools ? <FakeScalePanel /> : null}
          {devTools ? <ScanTestBox /> : null}
        </div>
        <PreviewPrinterPanel printouts={printouts} onClear={clear} />
      </div>
      {devTools ? <ScanSimulator /> : null}
    </section>
  );
}
