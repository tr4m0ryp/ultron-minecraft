#!/usr/bin/env bash
# PreToolUse hook for the player session: file tools may touch journal/ only.
# Exit 2 blocks the call and shows stderr to the model.
set -uo pipefail
journal="$(realpath -m "${BRAIN_JOURNAL:?}")"
input="$(cat)"
path="$(jq -r '.tool_input.file_path // .tool_input.path // empty' <<<"$input")"
[[ -z "$path" ]] && { echo "file tools are limited to journal/" >&2; exit 2; }
[[ "$path" != /* ]] && path="$(jq -r '.cwd // empty' <<<"$input")/$path"
case "$(realpath -m "$path")" in
  "$journal"/*) exit 0 ;;
  *) echo "only files under journal/ are yours; $path is not" >&2; exit 2 ;;
esac
