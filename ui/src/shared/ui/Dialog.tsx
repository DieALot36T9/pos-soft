import { useId, type ReactNode } from "react";

import { cn } from "../lib/utils";
import type { ButtonColour } from "./colours";

// The header colour says what the dialog is about (Design System): green for money in,
// red for removing, amber for waiting or approval, blue for finding, grey for settings.
const headerClasses: Record<ButtonColour, string> = {
  green: "border-green bg-green text-white",
  red: "border-red bg-red text-white",
  amber: "border-amber bg-amber-fill text-text",
  blue: "border-blue bg-blue text-white",
  purple: "border-purple bg-purple text-white",
  teal: "border-teal bg-teal text-white",
  grey: "border-grey bg-grey text-white",
};

type DialogProps = {
  title: string;
  colour: ButtonColour;
  children: ReactNode;
  // Wider dialogs for tables, such as the payment rows or the held sales list.
  width?: "normal" | "wide";
};

// A flat dialog over the screen: a coloured header and white content, no shadow and
// no movement. Back/Cancel goes bottom-left and the main action bottom-right.
export function Dialog({ title, colour, children, width = "normal" }: DialogProps) {
  const titleId = useId();
  return (
    <div className="fixed inset-0 z-40 flex items-center justify-center bg-text/50 p-4">
      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        className={cn(
          "flex max-h-full w-full flex-col overflow-auto rounded-ui border bg-panel",
          width === "wide" ? "max-w-3xl" : "max-w-md",
          headerClasses[colour].split(" ")[0],
        )}
      >
        <h2
          id={titleId}
          className={cn("rounded-t-ui border-b p-4 text-xl font-semibold", headerClasses[colour])}
        >
          {title}
        </h2>
        <div className="flex flex-col gap-3 p-4">{children}</div>
      </section>
    </div>
  );
}
