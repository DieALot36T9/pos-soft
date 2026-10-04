import type { AppInfo } from "../generated/AppInfo";
import { call } from "../shared/bridge";

// Asks Rust for the app's name and version. Proves the bridge works.
export const getAppInfo = (): Promise<AppInfo> => call<AppInfo>("app_info");
