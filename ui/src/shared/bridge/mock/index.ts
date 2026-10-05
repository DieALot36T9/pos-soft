import type { AppInfo } from "../../../generated/AppInfo";
import type { FakeScaleChange } from "../../../generated/FakeScaleChange";
import type { FakeScaleView } from "../../../generated/FakeScaleView";
import type { PaperWidth } from "../../../generated/PaperWidth";
import type { PreviewPrintout } from "../../../generated/PreviewPrintout";
import type { Reading } from "../../../generated/Reading";
import { mockScale, mockScaleRead, mockScaleSet } from "./scale";
import { testReceiptPicture } from "./testReceipt";

// What the fake Preview printer has printed and not yet handed out.
let waitingPrintouts: PreviewPrintout[] = [];

// The fake Rust side for `pnpm dev:mock` and tests, filled with demo data.
// Each answer is typed with the generated types, so it can't drift from Rust.
const handlers = {
  app_info: (): AppInfo => ({
    name: "Supershop POS",
    version: "0.1.0 (fake bridge)",
    dev_tools: true,
  }),
  hardware_test_print: (args?: Record<string, unknown>): null => {
    const paper = args?.paper as PaperWidth;
    waitingPrintouts.push({
      pictures: [testReceiptPicture(paper)],
      drawer_opened: false,
      saved_files: [`dev-prints/test-receipt-${paper}.png`],
    });
    return null;
  },
  preview_take_printouts: (): PreviewPrintout[] => {
    const printouts = waitingPrintouts;
    waitingPrintouts = [];
    return printouts;
  },
  print_window: (): null => null,
  dev_fake_scale_get: (): FakeScaleView => mockScale(),
  dev_fake_scale_set: (args?: Record<string, unknown>): FakeScaleView =>
    mockScaleSet(args?.change as FakeScaleChange),
  dev_fake_scale_read: (): Reading => mockScaleRead(),
} satisfies Record<string, (args?: Record<string, unknown>) => unknown>;

// Answers a command the way Rust would, after a short pause like a real call.
export async function mockCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const handler = (handlers as Record<string, (args?: Record<string, unknown>) => unknown>)[
    command
  ];
  if (!handler) {
    throw new Error(`The fake bridge has no command "${command}". Add it to shared/bridge/mock.`);
  }
  await Promise.resolve();
  return handler(args) as T;
}
