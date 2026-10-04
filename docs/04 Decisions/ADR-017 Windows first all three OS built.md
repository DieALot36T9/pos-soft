---
tags: [adr]
status: accepted
---
# ADR-017 Windows for shops, Mac for testing now, all three built

## Context
- Shops run **Windows 10** PCs with 16 GB RAM.
- The product owner currently has **only a Mac** to test on.
- The original requirement: the app must build for Linux, Windows and macOS.

## Decision
- **Development and the product owner's demo testing happen on macOS** until a Windows PC is available. Every milestone produces a Mac build (`.dmg`) to try.
- **Windows 10/11 is still the target for shops.** CI builds and runs every automated test on Windows on every pull request (every change reaches `main` through one), so Windows problems show up straight away.
- **Before UAT (M13/M14), everything is checked on a real Windows 10 PC with the shop's hardware.** The product owner needs a Windows PC by then.
- Linux builds and tests run in CI too.
- Printer connections: network (IP:9100) and serial work the same on every OS. USB printers use the Windows print spooler on Windows, and a raw CUPS queue on Mac/Linux.

## Consequences
- ✅ The product owner can try every milestone now, on their Mac.
- ✅ CI catches most Windows issues long before UAT.
- ⚠️ No printer, scanner or scale is available yet. Everything is built against fake hardware, and the first real-hardware test happens in M13 on Windows. This is the biggest remaining risk ([[Risks]] R1).
- ⚠️ USB receipt printers on macOS can be awkward (many have no Mac driver). If a printer is ever tested on the Mac, a **network** printer (or USB-serial) is the easiest. A USB-only printer needs a raw CUPS queue, or a direct-USB connection added later.
- ⚠️ Code signing is done later, once the product works. Mac demo builds are unsigned, so macOS shows a warning the first time. Open it with right-click → Open ([[Dev Environment Setup]]).
