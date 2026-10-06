#!/usr/bin/env bash
# Proves that `cargo deny check bans` (deny.toml) really stops one module crate
# depending on another (rule R1 in docs/03 Architecture/Module Rules.md).
#
# It works on a throw-away copy of the repo, so your files are never changed:
#   1. the real workspace passes;
#   2. `reports` may use a module in its tests (a dev-dependency) and still passes;
#   3. `catalog` depending on `sales` fails, and the error names `sales`;
#   4. `sales` depending on `db` fails.
# CI runs it on Linux. Run it yourself with:  bash scripts/check-module-bans.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COPY="$(mktemp -d)"
trap 'rm -rf "$COPY"' EXIT

say() { printf '\n== %s\n' "$*"; }

# Copy the Rust workspace only: no build output, no node_modules, no git history.
tar -C "$REPO_ROOT" --exclude=./target --exclude=./node_modules --exclude=./ui \
  --exclude=./.git -cf - . | tar -C "$COPY" -xf -

# Adds `<dependency>.workspace = true` under a section of crates/<crate>/Cargo.toml.
# Uses awk, which works the same on Linux and macOS.
add_dependency() {
  local crate="$1" section="$2" dependency="$3"
  local manifest="$COPY/crates/$crate/Cargo.toml"
  # Already there (reports uses sales in its tests): adding it again would be a duplicate key.
  if grep -q "^$dependency\.workspace = true" "$manifest"; then
    return
  fi
  if grep -q "^\[$section\]" "$manifest"; then
    awk -v header="[$section]" -v line="$dependency.workspace = true" \
      '{ print } $0 == header { print line }' "$manifest" >"$manifest.new"
    mv "$manifest.new" "$manifest"
  else
    printf '\n[%s]\n%s.workspace = true\n' "$section" "$dependency" >>"$manifest"
  fi
}

# Puts a crate's Cargo.toml (and the lock file) back the way they were.
restore() {
  cp "$REPO_ROOT/crates/$1/Cargo.toml" "$COPY/crates/$1/Cargo.toml"
  cp "$REPO_ROOT/Cargo.lock" "$COPY/Cargo.lock"
}

bans() {
  (cd "$COPY" && cargo deny --log-level error check bans 2>&1)
}

expect_pass() {
  say "$1: should pass"
  if ! output="$(bans)"; then
    echo "$output"
    echo "FAILED: cargo deny refused a workspace that follows the rules." >&2
    exit 1
  fi
  echo "ok"
}

expect_ban() {
  local banned="$2"
  say "$1: should fail"
  if output="$(bans)"; then
    echo "FAILED: cargo deny allowed it. Check the [bans] list in deny.toml." >&2
    exit 1
  fi
  if ! grep -q "crate '$banned" <<<"$output"; then
    echo "$output"
    echo "FAILED: cargo deny failed, but not because of the module \`$banned\`." >&2
    exit 1
  fi
  echo "ok, refused"
}

expect_pass "The real workspace"

add_dependency reports dev-dependencies sales
expect_pass "reports using sales in its tests only"
restore reports

add_dependency catalog dependencies sales
expect_ban "catalog depending on sales" sales
restore catalog

add_dependency sales dependencies db
expect_ban "sales depending on db" db
restore sales

say "cargo-deny keeps module crates apart."
