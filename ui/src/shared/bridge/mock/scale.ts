import type { AppError } from "../../../generated/AppError";
import type { FakeScaleChange } from "../../../generated/FakeScaleChange";
import type { FakeScaleView } from "../../../generated/FakeScaleView";
import type { Reading } from "../../../generated/Reading";

// The fake scale in the fake bridge. Rust checks and formats the weight; this
// stand-in only accepts plain numbers with at most 3 decimals.
let scale: FakeScaleView = { weight: "0.000", unit: "kg", stable: true, connected: true };

export const mockScale = (): FakeScaleView => scale;

export function mockScaleSet(change: FakeScaleChange): FakeScaleView {
  if (!/^-?\d+(\.\d{1,3})?$/.test(change.weight)) {
    const refused: AppError = {
      kind: "validation",
      field: "weight",
      code: "weight_invalid",
      params: {},
    };
    // Rust's invoke fails with a plain AppError object, so the fake does too.
    // eslint-disable-next-line @typescript-eslint/only-throw-error
    throw refused;
  }
  scale = {
    ...change,
    weight: change.weight.includes(".") ? change.weight : `${change.weight}.000`,
  };
  return scale;
}

export function mockScaleRead(): Reading {
  if (!scale.connected) {
    const timeout: AppError = {
      kind: "hardware",
      device: "scale",
      code: "timeout",
      detail: "waited 2000 ms",
    };
    // eslint-disable-next-line @typescript-eslint/only-throw-error
    throw timeout;
  }
  return {
    value_milli: 0,
    unit: scale.unit,
    stable: scale.stable,
    raw: `${scale.stable ? "ST" : "US"} ${scale.weight} ${scale.unit}`,
  };
}
