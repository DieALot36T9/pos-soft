import * as TooltipPrimitive from "@radix-ui/react-tooltip";
import type { ComponentProps } from "react";

import { cn } from "../lib/utils";

// The shadcn/ui tooltip, restyled flat: dark box, white text, 1 px border,
// no shadow, no animation, at most 280 px wide (Design System, "Tooltips").

// Shows tooltips after 0.4 s of hovering, or straight away with the Tab key.
export function TooltipProvider({ children }: { children: React.ReactNode }) {
  return <TooltipPrimitive.Provider delayDuration={400}>{children}</TooltipPrimitive.Provider>;
}

export const Tooltip = TooltipPrimitive.Root;
export const TooltipTrigger = TooltipPrimitive.Trigger;

export function TooltipContent({
  className,
  sideOffset = 6,
  ...props
}: ComponentProps<typeof TooltipPrimitive.Content>) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Content
        sideOffset={sideOffset}
        className={cn(
          "z-50 max-w-[280px] rounded-ui border border-text bg-text px-3 py-2 text-sm text-white",
          className,
        )}
        {...props}
      />
    </TooltipPrimitive.Portal>
  );
}
