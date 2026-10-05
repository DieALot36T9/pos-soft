import { useId } from "react";

import { Tip } from "./Tip";

type SelectFieldProps<T extends string> = {
  label: string;
  // Required: what the choice changes.
  tip: string;
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  error?: string | null;
};

// A labelled drop-down list with its tooltip and, when needed, an error under it.
export function SelectField<T extends string>({
  label,
  tip,
  value,
  options,
  onChange,
  error,
}: SelectFieldProps<T>) {
  const id = useId();
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={id} className="font-semibold">
        {label}
      </label>
      <Tip tip={tip}>
        <select
          id={id}
          value={value}
          onChange={(event) => onChange(event.target.value as T)}
          aria-invalid={error ? true : undefined}
          className={
            "min-h-10 rounded-ui border bg-panel px-3 " + (error ? "border-red" : "border-border")
          }
        >
          {options.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </Tip>
      {error ? <p className="text-red">{error}</p> : null}
    </div>
  );
}
