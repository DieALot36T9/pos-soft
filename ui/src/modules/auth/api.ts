import type { LoginResult } from "../../generated/LoginResult";
import type { RecoveryDone } from "../../generated/RecoveryDone";
import type { SecretCheck } from "../../generated/SecretCheck";
import type { SecretKind } from "../../generated/SecretKind";
import type { SessionView } from "../../generated/SessionView";
import type { UserTile } from "../../generated/UserTile";
import { call } from "../../shared/bridge";

// The active users, as tiles on the login screen (USR-01).
export const loginTiles = () => call<UserTile[]>("login_tiles");
// Checks a PIN or password in Rust. Five wrong tries lock the user for 60 s (USR-02).
export const login = (userId: string, secret: string) =>
  call<LoginResult>("login", { userId, secret });
export const logout = () => call<null>("logout");
// Locks the till; the same user unlocks it, or an admin helps (USR-03).
export const lockTill = () => call<SessionView>("lock");
export const unlock = (secret: string) => call<SecretCheck>("unlock", { secret });
export const unlockByAdmin = (username: string, password: string) =>
  call<SecretCheck>("unlock_by_admin", { username, password });
// Changes your own PIN or password after typing the current one (USR-07).
export const changeOwnSecret = (kind: SecretKind, current: string, newSecret: string) =>
  call<SecretCheck>("change_own_secret", { kind, current, newSecret });
// "Forgot admin password": the recovery code sets a new password (USR-09).
export const recoverAdmin = (
  code: string,
  username: string,
  password: string,
  passwordAgain: string,
) => call<RecoveryDone>("recover_admin", { code, username, password, passwordAgain });
