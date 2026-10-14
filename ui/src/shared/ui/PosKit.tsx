import React, { useEffect } from "react";
import type { LucideIcon } from "lucide-react";
import { X, Info, Delete } from "lucide-react";

export type Tone = "green" | "red" | "amber" | "blue" | "purple" | "teal" | "grey";

const BUTTON_TONES: Record<Tone, string> = {
  green: "bg-green text-white border-green",
  red: "bg-red text-white border-red",
  amber: "bg-amber-fill text-text border-amber font-semibold",
  blue: "bg-blue text-white border-blue",
  purple: "bg-purple text-white border-purple",
  teal: "bg-teal text-white border-teal",
  grey: "bg-panel text-text border-border hover:bg-grey-soft",
};

const SOFT_TONES: Record<Tone, string> = {
  green: "bg-green-soft text-green border-green",
  red: "bg-red-soft text-red border-red",
  amber: "bg-amber-soft text-amber border-amber",
  blue: "bg-blue-soft text-blue border-blue",
  purple: "bg-purple-soft text-purple border-purple",
  teal: "bg-teal-soft text-teal border-teal",
  grey: "bg-grey-soft text-grey border-border",
};

export interface KitButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  tone?: Tone;
  soft?: boolean;
  icon?: LucideIcon;
  shortcut?: string;
  tip: string;
  size?: "sm" | "md" | "till" | "pay";
}

export function KitButton({
  tone = "grey",
  soft = false,
  icon: Icon,
  shortcut,
  tip,
  size = "md",
  className = "",
  children,
  disabled,
  ...props
}: KitButtonProps) {
  const sizeClass = {
    sm: "min-h-9 px-3 py-1.5 text-sm gap-1.5",
    md: "min-h-10 px-4 py-2 text-sm font-medium gap-2",
    till: "min-h-14 px-4 py-2.5 text-base font-semibold gap-2",
    pay: "min-h-18 px-6 py-3 text-xl font-bold gap-3",
  }[size];

  const toneClass = soft ? SOFT_TONES[tone] : BUTTON_TONES[tone];

  return (
    <button
      type="button"
      title={shortcut ? `${tip} (${shortcut})` : tip}
      aria-label={typeof children === "string" ? children : tip}
      disabled={disabled}
      className={`inline-flex items-center justify-center rounded-ui border cursor-pointer select-none disabled:opacity-45 disabled:cursor-not-allowed ${toneClass} ${sizeClass} ${className}`}
      {...props}
    >
      {Icon && <Icon className={size === "pay" ? "w-6 h-6 shrink-0" : "w-4 h-4 shrink-0"} />}
      <span className="truncate">{children}</span>
      {shortcut && (
        <kbd
          className={`ml-auto px-1.5 py-0.5 text-xs font-mono rounded border ${
            !soft && tone !== "grey" && tone !== "amber"
              ? "bg-white/20 border-white/30 text-white"
              : "bg-black/10 border-black/20 text-current"
          }`}
        >
          {shortcut}
        </kbd>
      )}
    </button>
  );
}

export function PageHeader({
  breadcrumbs,
  title,
  description,
  actions,
}: {
  breadcrumbs?: string[];
  title: string;
  description: string;
  actions?: React.ReactNode;
}) {
  const slash = "/";
  return (
    <header className="flex flex-wrap items-start justify-between gap-4 pb-4 mb-4 border-b border-border">
      <div>
        {breadcrumbs && breadcrumbs.length > 0 && (
          <nav
            aria-label="Breadcrumb"
            className="flex items-center gap-1.5 text-xs font-medium text-text-muted mb-1"
          >
            {breadcrumbs.map((item, idx) => (
              <React.Fragment key={item}>
                {idx > 0 && <span>{slash}</span>}
                <span>{item}</span>
              </React.Fragment>
            ))}
          </nav>
        )}
        <h2 className="text-2xl font-bold text-text tracking-tight">{title}</h2>
        <p className="mt-1 text-sm text-text-muted max-w-3xl">{description}</p>
      </div>
      {actions && <div className="flex items-center gap-2 shrink-0">{actions}</div>}
    </header>
  );
}

export function KitField({
  label,
  description,
  example,
  required,
  error,
  children,
}: {
  label: string;
  description?: string;
  example?: string;
  required?: boolean;
  error?: string;
  children: React.ReactNode;
}) {
  const asterisk = "*";
  const egPrefix = "(e.g. ";
  const egSuffix = ")";
  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-1.5">
        <label className="text-sm font-semibold text-text">
          {label}
          {required && <span className="text-red ml-0.5">{asterisk}</span>}
        </label>
        {(description || example) && (
          <span
            title={`${description ?? ""}${example ? ` Example: ${example}` : ""}`}
            className="text-text-muted cursor-help inline-flex items-center"
          >
            <Info className="w-3.5 h-3.5" />
          </span>
        )}
      </div>
      {description && (
        <p className="text-xs text-text-muted leading-snug">
          {description}
          {example && (
            <span className="font-mono ml-1">
              {egPrefix}
              {example}
              {egSuffix}
            </span>
          )}
        </p>
      )}
      <div className="mt-0.5">{children}</div>
      {error && (
        <p
          role="alert"
          className="text-xs font-semibold text-red bg-red-soft border border-red rounded-ui px-2.5 py-1 mt-0.5"
        >
          {error}
        </p>
      )}
    </div>
  );
}

export function KitBadge({
  tone = "grey",
  icon: Icon,
  children,
}: {
  tone?: Tone;
  icon?: LucideIcon;
  children: React.ReactNode;
}) {
  return (
    <span
      className={`inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-ui text-xs font-semibold border ${SOFT_TONES[tone]}`}
    >
      {Icon && <Icon className="w-3.5 h-3.5 shrink-0" />}
      <span>{children}</span>
    </span>
  );
}

export function KitDrawer({
  open,
  onClose,
  title,
  description,
  footer,
  widthClass = "max-w-xl",
  children,
}: {
  open: boolean;
  onClose: () => void;
  title: string;
  description?: string;
  footer?: React.ReactNode;
  widthClass?: string;
  children: React.ReactNode;
}) {
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex justify-end bg-text/40" onClick={onClose}>
      <div
        role="dialog"
        aria-modal="true"
        aria-label={title}
        onClick={(e) => e.stopPropagation()}
        className={`w-full ${widthClass} h-full bg-panel border-l border-border flex flex-col justify-between`}
      >
        <div className="px-5 py-4 border-b border-border bg-bg flex items-start justify-between gap-3">
          <div>
            <h2 className="text-lg font-bold text-text">{title}</h2>
            {description && <p className="text-xs text-text-muted mt-0.5">{description}</p>}
          </div>
          <button
            type="button"
            onClick={onClose}
            title="Close panel (Esc)"
            className="p-2 rounded-ui border border-border bg-panel hover:bg-grey-soft text-text cursor-pointer"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
        <div className="p-5 overflow-y-auto flex-1 space-y-4">{children}</div>
        {footer && (
          <div className="px-5 py-3.5 border-t border-border bg-bg flex items-center justify-between gap-3">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}

export function KitNumpad({
  value,
  onChange,
  onEnter,
  allowDecimal = true,
  quickValues,
}: {
  value: string;
  onChange: (next: string) => void;
  onEnter?: () => void;
  allowDecimal?: boolean;
  quickValues?: { label: string; value: string }[];
}) {
  const press = (key: string) => {
    if (key === "C") {
      onChange("");
      return;
    }
    if (key === "BS") {
      onChange(value.slice(0, -1));
      return;
    }
    if (key === ".") {
      if (!allowDecimal || value.includes(".")) return;
      onChange(value === "" ? "0." : `${value}.`);
      return;
    }
    onChange(`${value}${key}`);
  };

  const keys = ["7", "8", "9", "4", "5", "6", "1", "2", "3", "C", "0", allowDecimal ? "." : "BS"];
  const backspaceLabel = "Backspace";
  const applyLabel = "Apply ↵";

  return (
    <div className="space-y-2.5">
      {quickValues && quickValues.length > 0 && (
        <div className="grid grid-cols-4 gap-2">
          {quickValues.map((q) => (
            <button
              key={q.label}
              type="button"
              onClick={() => onChange(q.value)}
              title={`Set value to ${q.label}`}
              className="h-10 px-2 rounded-ui border border-border bg-blue-soft text-blue font-semibold text-sm tabular-nums cursor-pointer"
            >
              {q.label}
            </button>
          ))}
        </div>
      )}
      <div className="grid grid-cols-3 gap-2">
        {keys.map((k) => (
          <button
            key={k}
            type="button"
            onClick={() => press(k)}
            title={k === "C" ? "Clear value" : k === "BS" ? "Backspace" : `Type ${k}`}
            className="h-12 rounded-ui border border-border bg-panel hover:bg-grey-soft text-lg font-bold text-text tabular-nums cursor-pointer flex items-center justify-center"
          >
            {k === "BS" ? <Delete className="w-5 h-5" /> : k}
          </button>
        ))}
      </div>
      {allowDecimal && (
        <div className="grid grid-cols-2 gap-2">
          <button
            type="button"
            onClick={() => press("BS")}
            title="Delete last digit (Backspace)"
            className="h-11 rounded-ui border border-border bg-panel hover:bg-grey-soft font-semibold text-sm flex items-center justify-center gap-1.5 cursor-pointer"
          >
            <Delete className="w-4 h-4" /> <span>{backspaceLabel}</span>
          </button>
          {onEnter && (
            <button
              type="button"
              onClick={onEnter}
              title="Apply value (Enter)"
              className="h-11 rounded-ui border border-green bg-green text-white font-bold text-sm flex items-center justify-center gap-1.5 cursor-pointer"
            >
              <span>{applyLabel}</span>
            </button>
          )}
        </div>
      )}
    </div>
  );
}
