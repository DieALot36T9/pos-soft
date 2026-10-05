import { useId, type ChangeEvent, type KeyboardEvent } from "react";

import { Tip } from "./Tip";

type FieldProps = {
  label: string;
  // Required: what to type, with an example. Example: "Price for one unit. Example: 120.00".
  tip: string;
  value: string;
  onChange: (value: string) => void;
  onKeyDown?: (event: KeyboardEvent<HTMLInputElement>) => void;
  // The message under the field when Rust refused what was typed.
  error?: string | null;
  // Narrow fields for short values such as a weight.
  width?: "short" | "full";
  // True for passwords: the letters are hidden.
  secret?: boolean;
};

// A labelled text box with its tooltip and, when needed, an error message under it.
export function Field({
  label,
  tip,
  value,
  onChange,
  onKeyDown,
  error,
  width = "full",
  secret = false,
}: FieldProps) {
  const id = useId();
  const errorId = `${id}-error`;
  const handleChange = (event: ChangeEvent<HTMLInputElement>) => onChange(event.target.value);
  return (
    <div className="flex flex-col gap-1">
      <label htmlFor={id} className="font-semibold">
        {label}
      </label>
      <Tip tip={tip}>
        <input
          id={id}
          type={secret ? "password" : "text"}
          value={value}
          onChange={handleChange}
          onKeyDown={onKeyDown}
          aria-invalid={error ? true : undefined}
          aria-describedby={error ? errorId : undefined}
          className={
            "min-h-10 rounded-ui border bg-panel px-3 text-base tabular-nums " +
            (error ? "border-red " : "border-border ") +
            (width === "short" ? "w-40" : "w-full")
          }
        />
      </Tip>
      {error ? (
        <p id={errorId} className="text-red">
          {error}
        </p>
      ) : null}
    </div>
  );
}
