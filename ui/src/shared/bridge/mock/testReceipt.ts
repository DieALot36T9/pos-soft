import type { PaperWidth } from "../../../generated/PaperWidth";
// The approved test receipt pictures, made by Rust (crates/app/tests/golden).
import receipt58 from "../../../../../crates/app/tests/golden/test_receipt_58mm.png";
import receipt80 from "../../../../../crates/app/tests/golden/test_receipt_80mm.png";

// The picture the real Preview printer shows after "Test print" on this paper.
export function testReceiptPicture(paper: PaperWidth): string {
  return paper === "mm58" ? receipt58 : receipt80;
}
