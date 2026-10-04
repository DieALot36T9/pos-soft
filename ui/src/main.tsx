import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { AppFrame } from "./app/AppFrame";
import { LanguageProvider } from "./shared/text";
import { TooltipProvider } from "./shared/ui/tooltip";
import "./shared/theme.css";

// Start-up: the providers every screen needs, then the app frame.
const rootElement = document.getElementById("root");
if (rootElement) {
  createRoot(rootElement).render(
    <StrictMode>
      <LanguageProvider language="en">
        <TooltipProvider>
          <AppFrame />
        </TooltipProvider>
      </LanguageProvider>
    </StrictMode>,
  );
}
