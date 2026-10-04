// The fixed colour code (Design System). One colour always means one thing.
export type ButtonColour = "green" | "red" | "amber" | "blue" | "purple" | "teal" | "grey";

// Strong colours fill buttons with white text. Amber is bright with dark text.
export const buttonColourClasses: Record<ButtonColour, string> = {
  green: "bg-green text-white border-green",
  red: "bg-red text-white border-red",
  amber: "bg-amber-fill text-text border-amber",
  blue: "bg-blue text-white border-blue",
  purple: "bg-purple text-white border-purple",
  teal: "bg-teal text-white border-teal",
  grey: "bg-grey text-white border-grey",
};

// How a disabled button looks, whatever its colour.
export const disabledClasses = "bg-grey-soft text-text-muted border-border cursor-not-allowed";
