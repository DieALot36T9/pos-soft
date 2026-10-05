// Codes the scan simulator offers. Test data, not screen text: the description
// of each one is in text/en.ts under scanner.samples.
export const sampleCodes = [
  { code: "5901234123457", kind: "product" },
  { code: "96385074", kind: "small" },
  // Default scale rules: "21" + item 12345 + 01.250 kg, and "23" + item 12345 + 099.50.
  { code: "2112345012506", kind: "weight" },
  { code: "2312345009951", kind: "price" },
  { code: "SKU-1001", kind: "sku" },
  { code: "5901234123450", kind: "wrong" },
] as const;
