import { useEffect, useState, type Dispatch, type SetStateAction } from "react";

import type { PreviewPrintout } from "../../generated/PreviewPrintout";
import { takePrintouts } from "./api";

// How often the screen asks the Preview printer for new printouts.
const CHECK_EVERY_MS = 1000;

type SetPrintouts = Dispatch<SetStateAction<PreviewPrintout[]>>;

// Asks for new printouts and puts them in front of the ones shown, newest first.
function checkForPrintouts(setPrintouts: SetPrintouts) {
  takePrintouts()
    .then((fresh) => {
      if (fresh.length > 0) {
        setPrintouts((shown) => [...fresh.reverse(), ...shown]);
      }
    })
    // A failed check is tried again on the next tick.
    .catch(() => undefined);
}

// The Preview printer's printouts, newest first. Checks for new ones every second,
// and at once when `checkNow` is called (after "Test print").
export function usePrintouts() {
  const [printouts, setPrintouts] = useState<PreviewPrintout[]>([]);

  useEffect(() => {
    const timer = setInterval(() => checkForPrintouts(setPrintouts), CHECK_EVERY_MS);
    return () => clearInterval(timer);
  }, []);

  return {
    printouts,
    checkNow: () => checkForPrintouts(setPrintouts),
    clear: () => setPrintouts([]),
  };
}
