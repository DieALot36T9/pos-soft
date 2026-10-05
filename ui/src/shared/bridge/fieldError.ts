// The field Rust named in a validation error, if any. Example: "minor_digits".
export function errorField(error: unknown): string | null {
  if (typeof error === "object" && error !== null && "field" in error) {
    const field = error.field;
    return typeof field === "string" ? field : null;
  }
  return null;
}
