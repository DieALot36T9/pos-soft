---
tags: [runbook, hardware]
---
# Scale Setup

## Connected checkout scale (live weight)
1. Connect the scale (RS-232 or USB-serial). Install the USB-serial driver if Windows doesn't show a COM port in Device Manager.
2. Settings → Hardware → Scale: choose the COM port and the serial settings from the scale's manual (often 9600, 8 data bits, no parity, 1 stop bit).
3. Choose a **protocol profile** (Generic continuous, CAS-type, Toledo-type). If unsure, start with Generic continuous.
4. Press **Read scale** with a known weight on it. The panel shows the parsed weight **and the raw data**.
5. If the weight is wrong or missing, try another profile, or create a custom one: set the request command (if the scale needs one) and the pattern that finds the number. The raw data shown helps.

## Label-printing scales (scale prints a barcode sticker)
1. On the scale, set the barcode format to EAN-13 with prefix **21** for weight (5-digit item code, weight in grams) or **23** for price, or whatever your scale uses.
2. In Settings → Barcode rules, make the rules match the scale (pattern like `21.....{NNDDD}`). Use the **test field**: scan a sticker and check it shows the right item code and weight/price.
3. On each weighed product, set the **Scale item code** to the item number used on the scale (e.g. `00123`).

## Troubleshooting
| Symptom | Fix |
|---|---|
| "Scale not responding" | Wrong COM port or baud; cable; scale not in PC mode |
| Weight never becomes "stable" | Profile has no stability flag → set "stable after N equal readings" |
| Weight 1000× too big/small | Unit in the profile (g vs kg) |
| Sticker not recognised | The rule pattern doesn't match the sticker. Use the test field |
Manual weight entry always works while the scale is being fixed.
