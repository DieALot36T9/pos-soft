import type { CashMoveInput } from "../../generated/CashMoveInput";
import type { CountLine } from "../../generated/CountLine";
import type { Money } from "../../generated/Money";
import type { ShiftClosed } from "../../generated/ShiftClosed";
import type { ShiftState } from "../../generated/ShiftState";
import type { ShiftView } from "../../generated/ShiftView";
import { call } from "../../shared/bridge";

// The till's open shift, the notes and coins, and whether the count comes first.
export const shiftState = () => call<ShiftState>("shift_current");
// Adds up a count by denomination, so the screen never adds money itself.
export const countTotal = (counts: CountLine[]) => call<Money>("shift_count_total", { counts });
// Opens a shift with a float, or with a count whose total is the float (SHF-01).
export const openShift = (float: Money, counts: CountLine[]) =>
  call<ShiftView>("shift_open", { float, counts });
// Cash in, paid-out or cash drop with a reason (SHF-03). May need an admin's approval.
export const moveCash = (input: CashMoveInput, approval: string | null) =>
  call<null>("shift_cash_move", { input, approval });
// Opens the drawer without a sale, with a reason (SHF-04). May need an admin's approval.
export const noSale = (reason: string, approval: string | null) =>
  call<null>("shift_no_sale", { reason, approval });
// Closes the cashier's own shift with the count (SHF-06).
export const closeShift = (counts: CountLine[], note: string | null) =>
  call<ShiftClosed>("shift_close", { counts, note });
// An admin closes a shift left open (SHF-07). The count is optional.
export const forceClose = (counts: CountLine[], note: string | null, approval: string | null) =>
  call<ShiftClosed>("shift_force_close", { counts, note, approval });
