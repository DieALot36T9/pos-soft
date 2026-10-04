---
tags: [adr]
status: accepted
---
# ADR-004 One crate per module

## Context
The product owner requires modules that are built and maintained independently, so that work on one module can't break another.

## Decision
Every module is its own crate in one Cargo workspace. Module crates depend **only on `core`**. Only `app` depends on modules and coordinates them through workflows. UI code mirrors this with one folder per module. See [[Module Rules]].

## Consequences
- ✅ The compiler makes cross-module imports impossible, and CI double-checks with `cargo deny check` (`wrappers = ["app"]`).
- ✅ Small crates rebuild quickly, and their tests run on their own (`cargo nextest -p catalog`).
- ✅ An AI developer only needs one module's page plus `core` to work safely on that module.
- ⚠️ Some data is passed through `app` (e.g. product prices given to pricing) instead of being read directly. This is deliberate; it keeps dependencies explicit.
- ⚠️ 19 crates means more `Cargo.toml` files. `xtask new-module` generates the skeleton.
