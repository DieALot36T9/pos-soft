import type { LucideIcon } from "lucide-react";

import { cn } from "../lib/utils";
import { buttonColourClasses, disabledClasses, type ButtonColour } from "./colours";
import { Tip } from "./Tip";

type ActionButtonProps = {
  // What the colour means: green = money in / finish, red = remove / cancel, … (Design System).
  colour: ButtonColour;
  icon: LucideIcon;
  // A verb and at most two more words. Example: "Hold sale".
  label: string;
  // Required: one sentence about what happens. Example: "Put this sale aside…".
  tip: string;
  shortcut?: string;
  // When set, the button can't be used and the tooltip says why. Example: "Add an item first."
  disabledReason?: string;
  // "till" buttons are 56 px tall; "office" buttons are 40 px.
  size?: "till" | "office";
  onClick: () => void;
};

// The main kind of button: colour + icon + label + shortcut + tooltip, all on one button.
export function ActionButton({
  colour,
  icon: Icon,
  label,
  tip,
  shortcut,
  disabledReason,
  size = "office",
  onClick,
}: ActionButtonProps) {
  const isDisabled = disabledReason !== undefined;
  // A disabled button stays focusable so its tooltip can still say why.
  const handleClick = () => {
    if (!isDisabled) {
      onClick();
    }
  };
  return (
    <Tip tip={disabledReason ?? tip} shortcut={isDisabled ? undefined : shortcut}>
      <button
        type="button"
        aria-disabled={isDisabled}
        onClick={handleClick}
        className={cn(
          "inline-flex items-center gap-2 rounded-ui border px-4 text-base font-semibold",
          size === "till" ? "min-h-14" : "min-h-10",
          isDisabled ? disabledClasses : buttonColourClasses[colour],
        )}
      >
        <Icon aria-hidden="true" size={20} />
        <span>{label}</span>
        {shortcut ? <kbd className="text-sm font-normal">{shortcut}</kbd> : null}
      </button>
    </Tip>
  );
}
