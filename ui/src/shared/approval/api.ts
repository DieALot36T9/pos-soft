import type { ApprovalResult } from "../../generated/ApprovalResult";
import type { Permission } from "../../generated/Permission";
import type { UserTile } from "../../generated/UserTile";
import { call } from "../bridge";

// The users who could approve: the login tiles of users with a password.
export const approverTiles = () => call<UserTile[]>("login_tiles");
// An admin's username and password allow one action for 60 seconds (USR-05).
export const requestOverride = (permission: Permission, username: string, password: string) =>
  call<ApprovalResult>("request_override", { permission, username, password });
