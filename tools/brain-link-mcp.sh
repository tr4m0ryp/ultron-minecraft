#!/usr/bin/env bash
# Launch the brain-link MCP server from any cwd: resolves the repo root from this script's
# own location and execs the release binary (build: cargo build --release -p brain-link).
# MCP_BIN overrides the binary path.
set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="${MCP_BIN:-$REPO/target/release/brain-link-mcp}"
if [[ ! -x "$BIN" ]]; then
  echo "brain-link-mcp not built: run 'cargo build --release -p brain-link' in $REPO" >&2
  exit 1
fi
exec "$BIN" "$@"
