#!/bin/bash
# Gets a Claude Code cloud session ready to build and test: Rust, cargo-nextest,
# Node/pnpm packages, Tauri's Linux packages and a first build.
# All the work is in scripts/cloud-setup.sh, which also works on a new dev box.
set -euo pipefail

# Only cloud sessions. On your own computer, run scripts/cloud-setup.sh yourself.
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

# Progress goes to stderr, so it can't be mistaken for hook output.
bash "$CLAUDE_PROJECT_DIR/scripts/cloud-setup.sh" >&2

# Make cargo tools (cargo-nextest) available to every command in the session.
if [ -n "${CLAUDE_ENV_FILE:-}" ]; then
  echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> "$CLAUDE_ENV_FILE"
fi
