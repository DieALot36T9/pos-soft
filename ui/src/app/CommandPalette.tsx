import { Search, CornerDownLeft, type LucideIcon } from "lucide-react";
import { useEffect, useRef, useState } from "react";

export interface CommandEntry {
  id: string;
  group: string;
  label: string;
  description: string;
  shortcut?: string;
  icon?: LucideIcon;
  onSelect: () => void;
}

const escLabel = "Esc";
const noMatchesPrefix = "No matching screens or actions found for “";
const noMatchesSuffix = "”.";

export function CommandPalette({
  open,
  onClose,
  entries,
}: {
  open: boolean;
  onClose: () => void;
  entries: CommandEntry[];
}) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) {
      setTimeout(() => {
        setQuery("");
        setSelectedIndex(0);
        inputRef.current?.focus();
      }, 0);
    }
  }, [open]);

  const filtered = entries.filter((item) => {
    const q = query.trim().toLowerCase();
    if (!q) return true;
    return (
      item.label.toLowerCase().includes(q) ||
      item.description.toLowerCase().includes(q) ||
      item.group.toLowerCase().includes(q)
    );
  });

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => (filtered.length ? (i + 1) % filtered.length : 0));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) =>
          filtered.length ? (i - 1 + filtered.length) % filtered.length : 0,
        );
      } else if (e.key === "Enter" && filtered[selectedIndex]) {
        e.preventDefault();
        filtered[selectedIndex].onSelect();
        onClose();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, filtered, selectedIndex, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-start justify-center pt-20 bg-black/40"
      onClick={onClose}
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label="Quick navigation and command search"
        onClick={(e) => e.stopPropagation()}
        className="w-full max-w-2xl bg-[var(--panel)] border border-[var(--border)] rounded-[6px] overflow-hidden"
      >
        <div className="flex items-center gap-3 px-4 py-3 border-b border-[var(--border)] bg-[var(--bg)]">
          <Search className="w-5 h-5 text-[var(--blue)] shrink-0" />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => {
              setQuery(e.target.value);
              setSelectedIndex(0);
            }}
            placeholder="Type a screen name or action (e.g. 'Products', 'Receive goods', 'Tax', 'Till')…"
            className="w-full bg-transparent text-base text-[var(--text)] placeholder:text-[var(--text-muted)] focus:outline-none"
          />
          <kbd className="px-2 py-0.5 text-xs font-mono bg-[var(--grey-soft)] border border-[var(--border)] rounded">
            <span>{escLabel}</span>
          </kbd>
        </div>

        <div className="max-h-96 overflow-y-auto divide-y divide-[var(--border)]">
          {filtered.length === 0 ? (
            <div className="p-8 text-center text-sm text-[var(--text-muted)]">
              {noMatchesPrefix}
              {query}
              {noMatchesSuffix}
            </div>
          ) : (
            filtered.map((item, idx) => {
              const active = idx === selectedIndex;
              return (
                <button
                  key={item.id}
                  type="button"
                  onMouseEnter={() => setSelectedIndex(idx)}
                  onClick={() => {
                    item.onSelect();
                    onClose();
                  }}
                  className={`w-full px-4 py-3 text-left flex items-center justify-between gap-3 cursor-pointer ${
                    active
                      ? "bg-[var(--blue-soft)] border-l-4 border-[var(--blue)]"
                      : "bg-[var(--panel)] hover:bg-[var(--bg)]"
                  }`}
                >
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-bold uppercase tracking-wider text-[var(--blue)]">
                        {item.group}
                      </span>
                      <span className="text-sm font-semibold text-[var(--text)]">{item.label}</span>
                    </div>
                    <p className="text-xs text-[var(--text-muted)] truncate mt-0.5">
                      {item.description}
                    </p>
                  </div>

                  <div className="flex items-center gap-2 shrink-0">
                    {item.shortcut && (
                      <kbd className="px-1.5 py-0.5 text-xs font-mono rounded border border-[var(--border)] bg-[var(--bg)] text-[var(--text)]">
                        {item.shortcut}
                      </kbd>
                    )}
                    {active && <CornerDownLeft className="w-4 h-4 text-[var(--blue)]" />}
                  </div>
                </button>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
}
