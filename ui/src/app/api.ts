import type { AppInfo } from "../generated/AppInfo";
import type { SignInResult } from "../generated/SignInResult";
import type { StartupView } from "../generated/StartupView";
import { call } from "../shared/bridge";

// Asks Rust for the app's name and version. Proves the bridge works.
export const getAppInfo = (): Promise<AppInfo> => call<AppInfo>("app_info");
// Which screen to open: recovery, setup, sign-in or the app (SET-01, BAK-06).
export const getStartup = () => call<StartupView>("startup_status");
// Checks a username and password in Rust (until the login screens of C-M3).
export const signIn = (username: string, password: string) =>
  call<SignInResult>("session_sign_in", { username, password });
export const signOut = () => call<null>("session_sign_out");
