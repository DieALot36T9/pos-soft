import { useState, type ReactNode } from "react";

import type { AppError } from "../../generated/AppError";
import type { Permission } from "../../generated/Permission";
import { ApprovalDialog } from "./ApprovalDialog";

type Asking = {
  permission: Permission;
  approved: (token: string) => void;
  cancelled: () => void;
};

// True when Rust says an admin may approve this one action (AppError "override_required").
function needsApproval(error: unknown): error is Extract<AppError, { kind: "override_required" }> {
  return (
    typeof error === "object" &&
    error !== null &&
    "kind" in error &&
    error.kind === "override_required"
  );
}

// Runs an action that may need an admin's approval (USR-05). `work` gets the approval
// token: null the first time. When Rust answers "override_required", the approval dialog
// opens, and once an admin allows it, `work` runs again with the single-use token.
// Cancelling fails the action with Rust's original answer. Show `dialog` on the page.
export function useApproval(): {
  withApproval: <T>(work: (approval: string | null) => Promise<T>) => Promise<T>;
  dialog: ReactNode;
} {
  const [asking, setAsking] = useState<Asking | null>(null);

  const withApproval = <T,>(work: (approval: string | null) => Promise<T>): Promise<T> =>
    work(null).catch((error: unknown) => {
      if (!needsApproval(error)) {
        throw error;
      }
      return new Promise<T>((resolve, reject) => {
        setAsking({
          permission: error.permission,
          approved: (token) => {
            setAsking(null);
            work(token).then(resolve, reject);
          },
          cancelled: () => {
            setAsking(null);
            // Rust's answers are plain AppError objects, as in every other failed call.
            // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
            reject(error);
          },
        });
      });
    });

  const dialog = asking ? (
    <ApprovalDialog
      permission={asking.permission}
      onApproved={asking.approved}
      onCancel={asking.cancelled}
    />
  ) : null;

  return { withApproval, dialog };
}
