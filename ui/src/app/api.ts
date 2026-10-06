import type { AppInfo } from "../generated/AppInfo";
import type { SessionView } from "../generated/SessionView";
import type { StartupView } from "../generated/StartupView";
import { call } from "../shared/bridge";

// Asks Rust for the app's name and version. Proves the bridge works.
export const getAppInfo = (): Promise<AppInfo> => call<AppInfo>("app_info");
// Which screen to open: recovery, setup, sign-in or the app (SET-01, BAK-06).
export const getStartup = () => call<StartupView>("startup_status");
// Logs out (USR-01). The login screen shows next.
export const logOut = () => call<null>("logout");
// Locks the till: F12, the Lock button or the idle timer (USR-03).
export const lockTill = () => call<SessionView>("lock");
// The till's open shift: after login, a cashier with an open shift goes to Checkout (USR-01).
export { shiftState } from "../modules/shifts/api";
