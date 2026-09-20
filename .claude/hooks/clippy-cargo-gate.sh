#!/usr/bin/env bash
# clippy-cargo-gate.sh
#
# Stop hook: runs `cargo clippy` on this project, but only if this
# session's transcript shows a `cargo build` or `cargo run` command was
# actually run. Complements the sibling Stop hook that always checks
# modified .rs files via `git diff` regardless of session activity --
# this one only fires when a real build/run happened this session.
#
# Recreated after the original file was found missing from disk; the only
# surviving trace was the hooks-config doc at
# resources/ee/dds/skills/clippy_warnings.md, which references this path
# but never contained the script itself. A backup copy of this recreation
# lives alongside that doc at resources/ee/dds/skills/clippy-cargo-gate.sh.

set -uo pipefail

input="$(cat)"
transcript_path="$(printf '%s' "$input" | jq -r '.transcript_path // empty' 2>/dev/null || true)"

[ -z "$transcript_path" ] && exit 0
[ -f "$transcript_path" ] || exit 0

if ! grep -Eq '"command"[[:space:]]*:[[:space:]]*"[^"]*cargo[[:space:]]+(build|run)\b' "$transcript_path"; then
    exit 0
fi

command -v cargo >/dev/null 2>&1 || exit 0

out=$(cargo clippy --message-format=short 2>&1)
# cargo always prints "Checking .../Finished ... in X.XXs" even on clean
# code, so a bare non-empty check would false-positive every time. Only
# the aggregate "warning: ... generated N warnings" summary line (or a
# direct compiler error) means there's something to report.
issues=$(printf '%s\n' "$out" | grep -E '^(warning|error)(\[|:)' || true)
[ -z "$issues" ] && exit 0

msg="cargo clippy found issues (this session ran cargo build/run):"$'\n'"$out"
jq -n --arg ctx "$msg" '{hookSpecificOutput:{hookEventName:"Stop", additionalContext:$ctx}}' 2>/dev/null || true
