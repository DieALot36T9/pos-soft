import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { AppFrame } from "./app/AppFrame";
import "./app/i18n";
import { TooltipProvider } from "./shared/ui/tooltip";
import "./shared/theme.css";

// Start-up: screen text (i18next), the providers every screen needs, then the app frame.
const rootElement = document.getElementById("root");
if (rootElement) {
  createRoot(rootElement).render(
    <StrictMode>
      <TooltipProvider>
        <AppFrame />
      </TooltipProvider>
    </StrictMode>,
  );
}
