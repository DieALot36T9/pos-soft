---
tags: [guide, release]
---
# Release Process

## Versioning
Semantic versioning: `MAJOR.MINOR.PATCH`.
- PATCH: bug fixes, no migrations.
- MINOR: features; migrations allowed (always forward-compatible).
- MAJOR: a breaking change for users (e.g. a settings change that needs a manual step).

## Checklist
1. All milestone stories done; `CHANGELOG.md` *Unreleased* section complete.
2. `cargo xtask bench` within budgets; `cargo xtask powercut` passes.
3. **Migration test:** upgrade a copy of the previous release's `uat` database; run the scripted-day test on it.
4. Bump the version in `Cargo.toml` (workspace), `package.json` and `tauri.conf.json` (`cargo xtask bump 1.2.0` does all three).
5. Move *Unreleased* to the new version with today's date.
6. Tag `v1.2.0` → `release.yml` builds Windows (.msi + .exe, **signed**), Linux (.AppImage + .deb) and macOS (.dmg, signed + notarized if the account exists) and creates a draft GitHub release.
7. Install the Windows build on a clean VM: fresh setup works, upgrade over the previous version works, the upgrade took a `pre-migrate` backup.
8. Product owner approves → publish the release.

## Updating a shop (offline)
Copy the new installer to the till (USB). Run it. On start, the app backs up, migrates and opens. The [[Installation]] runbook has the steps.

## Signing
Signing is set up **later, once the product works** (before the pilot). Until then: on a Mac, right-click the app → **Open**; on Windows, SmartScreen → **More info** → **Run anyway**.
- Windows: a code-signing certificate (OV/EV) or Azure Trusted Signing. Secrets live in GitHub Actions secrets, owned by the product owner.
- macOS: Apple Developer ID + notarization (only if macOS is distributed).
