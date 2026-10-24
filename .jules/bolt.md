## 2026-10-24 - [Debouncing Barcode Scanners]
**Learning:** Barcode scanners in this POS system act as rapid keyboards. Without debouncing on search inputs, a single scan (e.g. 13 digits) fires sequential API requests for each character.
**Action:** Always implement an 80ms debounce on search-as-you-type inputs that receive scanner input to coalesce the scan into a single API request.
