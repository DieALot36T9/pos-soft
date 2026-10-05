import { invoke } from "@tauri-apps/api/core";

// The fake bridge runs in `pnpm dev:mock` (a normal browser) and in tests.
const useFakeBridge = import.meta.env.MODE === "mock" || import.meta.env.MODE === "test";

// Calls a Rust command and returns its answer. Only a module's api.ts calls this;
// components never do. If Rust refuses, the promise fails with an AppError.
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (useFakeBridge) {
    // Loaded only in the fake bridge modes, so the real app never carries the demo data.
    const { mockCall } = await import("./mock");
    return mockCall<T>(command, args);
  }
  return invoke<T>(command, args);
}

export { errorMessage } from "./errors";
export { errorField } from "./fieldError";
