import { invoke } from "@tauri-apps/api/core";

import { mockCall } from "./mock";

// The fake bridge runs in `pnpm dev:mock` (a normal browser) and in tests.
const useFakeBridge = import.meta.env.MODE === "mock" || import.meta.env.MODE === "test";

// Calls a Rust command and returns its answer. Only a module's api.ts calls this;
// components never do. If Rust refuses, the promise fails with an AppError.
export function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (useFakeBridge) {
    return mockCall<T>(command, args);
  }
  return invoke<T>(command, args);
}

export { errorMessage } from "./errors";
