import type { LucideIcon } from "lucide-react";

import { cn } from "../lib/utils";
import type { ButtonColour } from "./colours";
import { Tip } from "./Tip";

// Soft colour behind the strong colour's text and icon (Design System).
const badgeColourClasses: Record<ButtonColour, string> = {
  green: "bg-green-soft text-green border-green",
  red: "bg-red-soft text-red border-red",
  amber: "bg-amber-soft text-amber border-amber",
  blue: "bg-blue-soft text-blue border-blue",
  purple: "bg-purple-soft text-purple border-purple",
  teal: "bg-teal-soft text-teal border-teal",
  grey: "bg-grey-soft text-grey border-grey",
};

type BadgeProps = {
  colour: ButtonColour;
  icon: LucideIcon;
  // A short word, so colour is never the only signal. Example: "Still".
  word: string;
  tip: string;
};

// A small status mark: soft colour + icon + word, with a tooltip.
export function Badge({ colour, icon: Icon, word, tip }: BadgeProps) {
  return (
    <Tip tip={tip}>
      <span
        tabIndex={0}
        className={cn(
          "inline-flex items-center gap-1 rounded-ui border px-2 py-0.5 text-sm font-semibold",
          badgeColourClasses[colour],
        )}
      >
        <Icon aria-hidden="true" size={16} />
        {word}
      </span>
    </Tip>
  );
}
