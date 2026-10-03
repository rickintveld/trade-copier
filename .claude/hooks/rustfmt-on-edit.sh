#!/usr/bin/env bash
# PostToolUse hook: format Rust files after Claude edits them.
input=$(cat)
file_path=$(printf '%s' "$input" | jq -r '.tool_input.file_path // empty')
case "$file_path" in
  *.rs) rustfmt --edition 2021 "$file_path" 2>/dev/null || true ;;
esac
exit 0
