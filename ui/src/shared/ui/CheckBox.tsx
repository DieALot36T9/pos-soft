import { useId } from "react";

import { Tip } from "./Tip";

type CheckBoxProps = {
  label: string;
  // Required: what ticking it does.
  tip: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
};

// A tick box with its label and tooltip.
export function CheckBox({ label, tip, checked, onChange }: CheckBoxProps) {
  const id = useId();
  return (
    <div className="flex items-center gap-2">
      <Tip tip={tip}>
        <input
          id={id}
          type="checkbox"
          checked={checked}
          onChange={(event) => onChange(event.target.checked)}
          className="size-5 accent-blue"
        />
      </Tip>
      <label htmlFor={id}>{label}</label>
    </div>
  );
}
