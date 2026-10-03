#!/usr/bin/env bash
# PreToolUse hook for Edit/Write/NotebookEdit: blocks writes outside the given
# path prefixes (relative to the project root). Used by the domain agents so
# they can read across the codebase but only modify their own domain.
#
# Usage: guard-scope.sh <prefix> [<prefix> ...]
#   A prefix ending in "/" matches a directory, otherwise an exact file.
set -euo pipefail

input=$(cat)
file_path=$(printf '%s' "$input" | jq -r '.tool_input.file_path // .tool_input.notebook_path // empty')
[ -z "$file_path" ] && exit 0

root="${CLAUDE_PROJECT_DIR:-$(pwd)}"
case "$file_path" in
  "$root"/*) rel="${file_path#"$root"/}" ;;
  /*) echo "Blocked: $file_path is outside the project." >&2; exit 2 ;;
  *) rel="$file_path" ;;
esac

if [[ "$rel" == *..* ]]; then
  echo "Blocked: path traversal in '$rel'." >&2
  exit 2
fi

for prefix in "$@"; do
  case "$prefix" in
    */) [[ "$rel" == "$prefix"* ]] && exit 0 ;;
    *) [[ "$rel" == "$prefix" ]] && exit 0 ;;
  esac
done

echo "Blocked: '$rel' is outside this agent's write scope ($*). Read it if needed, but hand the change off to the owning agent and describe exactly what must change." >&2
exit 2
