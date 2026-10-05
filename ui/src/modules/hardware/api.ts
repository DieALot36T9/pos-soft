import type { FakeScaleChange } from "../../generated/FakeScaleChange";
import type { FakeScaleView } from "../../generated/FakeScaleView";
import type { PaperWidth } from "../../generated/PaperWidth";
import type { PreviewPrintout } from "../../generated/PreviewPrintout";
import type { Reading } from "../../generated/Reading";
import { call } from "../../shared/bridge";

// Prints the sample receipt on the receipt printer (the Preview printer for now).
export const testPrint = (paper: PaperWidth): Promise<null> =>
  call<null>("hardware_test_print", { paper });

// Every Preview printer printout since the last call, oldest first.
export const takePrintouts = (): Promise<PreviewPrintout[]> =>
  call<PreviewPrintout[]>("preview_take_printouts");

// Opens the computer's print dialog for what the window shows.
export const printWindow = (): Promise<null> => call<null>("print_window");

// Dev only: what the fake scale holds.
export const getFakeScale = (): Promise<FakeScaleView> => call<FakeScaleView>("dev_fake_scale_get");

// Dev only: changes the fake scale. Rust checks the typed weight.
export const setFakeScale = (change: FakeScaleChange): Promise<FakeScaleView> =>
  call<FakeScaleView>("dev_fake_scale_set", { change });

// Dev only: reads the fake scale the way checkout will read a real one.
export const readFakeScale = (): Promise<Reading> => call<Reading>("dev_fake_scale_read");
