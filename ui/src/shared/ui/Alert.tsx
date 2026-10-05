import { AlertTriangle, CheckCircle, Info, XCircle, type LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

import { cn } from "../lib/utils";

type AlertKind = "problem" | "warning" | "done" | "info";

// One colour and one icon per kind of message (Design System).
const looks: Record<AlertKind, { classes: string; icon: LucideIcon; role: "alert" | "status" }> = {
  problem: { classes: "border-red bg-red-soft text-red", icon: XCircle, role: "alert" },
  warning: { classes: "border-amber bg-amber-soft text-amber", icon: AlertTriangle, role: "alert" },
  done: { classes: "border-green bg-green-soft text-green", icon: CheckCircle, role: "status" },
  info: { classes: "border-blue bg-blue-soft text-blue", icon: Info, role: "status" },
};

// A message box: red for a problem, amber for a warning, green when done, blue for information.
export function Alert({ kind, children }: { kind: AlertKind; children: ReactNode }) {
  const { classes, icon: Icon, role } = looks[kind];
  return (
    <div role={role} className={cn("flex items-start gap-2 rounded-ui border p-3", classes)}>
      <Icon aria-hidden="true" size={20} className="mt-0.5 shrink-0" />
      <div>{children}</div>
    </div>
  );
}
