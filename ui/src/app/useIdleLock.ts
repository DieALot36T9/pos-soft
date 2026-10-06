import { useEffect } from "react";

// Locks the till after this many minutes without a key or mouse press (USR-03).
// The timer starts again at every press.
export function useIdleLock(idleLockMinutes: number, lock: () => void) {
  useEffect(() => {
    let timer = window.setTimeout(lock, idleLockMinutes * 60_000);
    const restart = () => {
      window.clearTimeout(timer);
      timer = window.setTimeout(lock, idleLockMinutes * 60_000);
    };
    const events = ["keydown", "mousedown", "touchstart"] as const;
    events.forEach((name) => window.addEventListener(name, restart));
    return () => {
      window.clearTimeout(timer);
      events.forEach((name) => window.removeEventListener(name, restart));
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- restart only when the setting changes.
  }, [idleLockMinutes]);
}
