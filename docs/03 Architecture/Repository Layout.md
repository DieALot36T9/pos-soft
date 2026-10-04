---
tags: [architecture]
---
# Repository Layout

```
supershop-pos/
├── CLAUDE.md                     # rules for the AI developer (points here)
├── CONTRIBUTING.md               # short; links to docs/09 Guides/Coding Standards
├── Cargo.toml                    # workspace, shared lints and profiles
├── rust-toolchain.toml           # pinned stable toolchain
├── package.json, pnpm-lock.yaml  # UI workspace
├── crates/
│   ├── core/        src/{lib.rs, money.rs, qty.rs, bps.rs, id.rs, time.rs, error.rs, permission.rs, rounding.rs}
│   ├── db/          src/{lib.rs, open.rs, migrate.rs, backup.rs, restore.rs}
│   ├── settings/    CLAUDE.md  src/  migrations/  tests/contract.rs   (every module crate has its own CLAUDE.md)
│   ├── auth/  audit/  tax/  catalog/  pricing/  barcode/  inventory/
│   ├── sales/  shifts/  customers/  purchasing/  reports/  io/
│   ├── printing/    src/  fonts/ (one open-licence font)  tests/golden/*.png
│   ├── hardware/    src/{printer/, scale/, discover.rs}
│   └── app/
│       ├── src/{main.rs, state.rs, session.rs, guard.rs, jobs.rs, migrations.rs}
│       ├── src/commands/   # one file per UI area
│       ├── src/workflows/  # one file per use case
│       ├── tests/          # workflow integration tests
│       ├── tauri.conf.json, capabilities/, icons/
├── ui/                           # see Frontend Architecture
├── xtask/                        # cargo xtask: check-deps, gen-types, seed, bench, release-notes
├── docs/                         # this Obsidian vault
└── .github/workflows/
    ├── ci.yml                    # fmt, clippy, tests, check-deps, types up to date, ui lint/test (3 OS)
    └── release.yml               # tauri-action builds + signing on tag
```

## Workspace settings (root `Cargo.toml`)
```toml
[workspace.lints.rust]
unsafe_code = "deny"            # hardware crate overrides to "allow" for Windows FFI only

[workspace.lints.clippy]
unwrap_used = "deny"
float_arithmetic = "warn"       # "deny" in core, tax, pricing, sales
too_many_lines = "warn"

[profile.dev]
opt-level = 0
[profile.dev.package."*"]
opt-level = 1                   # deps optimized, so dev builds aren't sluggish

[profile.release]
lto = "thin"
codegen-units = 1
opt-level = 3
strip = true
```
