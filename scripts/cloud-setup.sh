#!/usr/bin/env bash
# Get a fresh Linux machine (a Claude cloud session or a new dev box) ready to
# build and test the whole project: Rust, Node and pnpm, cargo-nextest, cargo-deny,
# and the Linux packages that Tauri needs. Safe to run more than once.
#
# Use it as the cloud environment's setup script:  bash scripts/cloud-setup.sh
set -euo pipefail

NEXTEST_VERSION="0.9.146"
DENY_VERSION="0.20.2"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

say() { printf '\n== %s\n' "$*"; }

# Run a command as root. Cloud sessions already run as root; dev boxes use sudo.
as_root() {
  if [ "$(id -u)" -eq 0 ]; then "$@"; else sudo "$@"; fi
}

install_linux_packages() {
  say "Linux packages for Tauri and serial ports"
  local packages=(
    build-essential pkg-config curl file
    libwebkit2gtk-4.1-dev libssl-dev libayatana-appindicator3-dev
    librsvg2-dev libudev-dev libxdo-dev
  )
  # Only call apt when something is missing, because apt is slow.
  local missing=()
  for package in "${packages[@]}"; do
    dpkg -s "$package" >/dev/null 2>&1 || missing+=("$package")
  done
  if [ "${#missing[@]}" -eq 0 ]; then
    echo "All packages already installed."
    return
  fi
  as_root apt-get update -y
  as_root env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends "${missing[@]}"
}

install_rust() {
  say "Rust (the version pinned in rust-toolchain.toml)"
  if ! command -v rustup >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
  fi
  # Running rustup inside the repo installs the pinned toolchain and its components.
  (cd "$REPO_ROOT" && rustup show active-toolchain || rustup toolchain install)
}

install_nextest() {
  say "cargo-nextest $NEXTEST_VERSION"
  if command -v cargo-nextest >/dev/null 2>&1 && cargo nextest --version | grep -q "$NEXTEST_VERSION"; then
    echo "Already installed."
    return
  fi
  local url="https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-${NEXTEST_VERSION}/cargo-nextest-${NEXTEST_VERSION}-x86_64-unknown-linux-gnu.tar.gz"
  # The prebuilt download is fast. Building from source is the fallback.
  if ! curl -fsSL "$url" | tar -xz -C "${CARGO_HOME:-$HOME/.cargo}/bin"; then
    cargo install cargo-nextest --version "$NEXTEST_VERSION" --locked
  fi
}

install_cargo_deny() {
  say "cargo-deny $DENY_VERSION"
  if command -v cargo-deny >/dev/null 2>&1 && cargo deny --version | grep -q "$DENY_VERSION"; then
    echo "Already installed."
    return
  fi
  local name="cargo-deny-${DENY_VERSION}-x86_64-unknown-linux-musl"
  local url="https://github.com/EmbarkStudios/cargo-deny/releases/download/${DENY_VERSION}/${name}.tar.gz"
  # The prebuilt download is fast. Building from source is the fallback.
  if ! curl -fsSL "$url" | tar -xz -C "${CARGO_HOME:-$HOME/.cargo}/bin" --strip-components=1 "${name}/cargo-deny"; then
    cargo install cargo-deny --version "$DENY_VERSION" --locked
  fi
}

install_node_tools() {
  say "Node.js and pnpm"
  if ! command -v node >/dev/null 2>&1; then
    echo "Node.js is missing. Install Node.js 22 LTS, then run this script again." >&2
    exit 1
  fi
  if ! command -v pnpm >/dev/null 2>&1; then
    corepack enable
  fi
  (cd "$REPO_ROOT" && pnpm install --frozen-lockfile)
}

warm_up_build() {
  say "First build (so the first test run in the session is quick)"
  (cd "$REPO_ROOT" && cargo build --workspace --all-targets)
}

install_linux_packages
install_rust
install_nextest
install_cargo_deny
install_node_tools
warm_up_build
say "Ready. Try: cargo nextest run --workspace,  cargo deny check  and  pnpm test"
