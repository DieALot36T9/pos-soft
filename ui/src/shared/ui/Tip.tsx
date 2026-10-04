import type { ReactElement } from "react";

import { Tooltip, TooltipContent, TooltipTrigger } from "./tooltip";

type TipProps = {
  // One plain sentence about what happens. Comes from the module's text/en.ts.
  tip: string;
  // The shortcut key, if there is one. Example: "F5".
  shortcut?: string;
  // The one element the tooltip belongs to, such as a button or a field.
  children: ReactElement;
};

// A floating tooltip for a button, field or status light. Every one of them has a tip.
export function Tip({ tip, shortcut, children }: TipProps) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent>
        {tip}
        {shortcut ? <kbd className="ml-2 font-semibold">{shortcut}</kbd> : null}
      </TooltipContent>
    </Tooltip>
  );
}
