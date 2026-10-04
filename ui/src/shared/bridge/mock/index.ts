import type { AppInfo } from "../../../generated/AppInfo";

// The fake Rust side for `pnpm dev:mock` and tests, filled with demo data.
// Each answer is typed with the generated types, so it can't drift from Rust.
const handlers = {
  app_info: (): AppInfo => ({ name: "Supershop POS", version: "0.1.0 (fake bridge)" }),
} satisfies Record<string, (args?: Record<string, unknown>) => unknown>;

// Answers a command the way Rust would, after a short pause like a real call.
export async function mockCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const handler = (handlers as Record<string, (args?: Record<string, unknown>) => unknown>)[
    command
  ];
  if (!handler) {
    throw new Error(`The fake bridge has no command "${command}". Add it to shared/bridge/mock.`);
  }
  await Promise.resolve();
  return handler(args) as T;
}
