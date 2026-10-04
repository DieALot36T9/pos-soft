import type { LucideIcon } from "lucide-react";

import { cn } from "../lib/utils";
import { buttonColourClasses, type ButtonColour } from "./colours";
import { Tip } from "./Tip";

type IconButtonProps = {
  icon: LucideIcon;
  // Required: says what the button does. Also read out by screen readers.
  tip: string;
  shortcut?: string;
  colour?: ButtonColour;
  onClick: () => void;
};

// A small icon-only button, for the back office only. Never used on the till.
export function IconButton({
  icon: Icon,
  tip,
  shortcut,
  colour = "grey",
  onClick,
}: IconButtonProps) {
  return (
    <Tip tip={tip} shortcut={shortcut}>
      <button
        type="button"
        aria-label={tip}
        onClick={onClick}
        className={cn(
          "inline-flex min-h-10 min-w-10 items-center justify-center rounded-ui border",
          buttonColourClasses[colour],
        )}
      >
        <Icon aria-hidden="true" size={20} />
      </button>
    </Tip>
  );
}
