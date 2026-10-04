---
tags: [runbook]
---
# Installation

## Before you start
- Windows 10/11 64-bit, 4 GB RAM minimum (16 GB recommended), SSD recommended.
- Turn on **BitLocker** (or another disk encryption) if the PC could be stolen.
- Set the correct **time zone and clock**. Business dates depend on it.
- Install the receipt printer's Windows driver if it connects by USB ([[Printer Setup]]).
- Have a USB drive ready for the second backup folder.

## Install
1. Run `SupershopPOS_x.y.z_x64-setup.exe` (or the `.msi`). If WebView2 is missing, the installer adds it (internet needed only for that; otherwise use the offline WebView2 installer from Microsoft).
2. Start the app → the [[Setup Wizard]] opens.
3. Print and store the **recovery code** somewhere safe (not near the till).
4. Complete the hardware step and print a test receipt.
5. Create cashier users.
6. Import products or add them.

## Upgrade
1. Close the app (an on-close backup is made).
2. Run the new installer.
3. Start the app. It backs up (`pre-migrate`) and upgrades the database automatically.
4. Print a test receipt and check today's sales are present.

## Uninstall
Uninstalling **keeps** the database and backups in the app data folder. Delete that folder only if you really want to remove all data.

## Moving to a new PC
1. On the old PC: Back office → Backups → Back up now (to USB).
2. Install on the new PC. In the setup wizard's first screen choose **"Restore from a backup"** instead of starting fresh.
3. Re-do the hardware settings (printers and ports differ).
