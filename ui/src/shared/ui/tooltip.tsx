import * as TooltipPrimitive from "@radix-ui/react-tooltip";
import type { ComponentProps } from "react";

import { cn } from "../lib/utils";

// Shows tooltips after 0.4 s of hovering, or straight away with the Tab key.
// Positioned on top (`side="top"`) with pointer-events-none so it never covers inputs while typing.
export function TooltipProvider({ children }: { children: React.ReactNode }) {
  return <TooltipPrimitive.Provider delayDuration={400}>{children}</TooltipPrimitive.Provider>;
}

export const Tooltip = TooltipPrimitive.Root;
export const TooltipTrigger = TooltipPrimitive.Trigger;

export function TooltipContent({
  className,
  side = "top",
  align = "start",
  sideOffset = 6,
  ...props
}: ComponentProps<typeof TooltipPrimitive.Content>) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Content
        side={side}
        align={align}
        sideOffset={sideOffset}
        collisionPadding={8}
        className={cn(
          "pointer-events-none z-50 max-w-[280px] rounded-ui border border-text bg-text px-3 py-1.5 text-xs text-white",
          className,
        )}
        {...props}
      />
    </TooltipPrimitive.Portal>
  );
}
