// Runs before every UI test: adds matchers such as toBeInTheDocument(),
// and clears the screen after each test so tests don't see each other's output.
import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach } from "vitest";

afterEach(() => {
  cleanup();
});
