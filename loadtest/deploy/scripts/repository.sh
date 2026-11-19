#!/usr/bin/env bash
set -euo pipefail

target="$1"
url="$2"
ref="${3:-}"

if [[ -e "$target/.git" ]]; then
  printf 'using repository: %s\n' "$target"
  exit 0
fi

if [[ -e "$target" ]]; then
  printf 'error: repository path exists but is not a Git repository: %s\n' "$target" >&2
  exit 1
fi

mkdir -p "$(dirname "$target")"
args=(clone)
[[ -n "$ref" ]] && args+=(--branch "$ref")
git "${args[@]}" "$url" "$target"
