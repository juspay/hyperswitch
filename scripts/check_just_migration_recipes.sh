#!/usr/bin/env bash
set -euo pipefail

if ! command -v just >/dev/null 2>&1; then
    echo "Error: 'just' is required to validate migration recipes"
    exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
default_db_url='postgresql://db_user:db_pass@localhost:5432/hyperswitch_db'

compatible_output="$(just --justfile "${repo_root}/justfile" --dry-run migrate_v2_compatible 2>&1)"

if ! grep -Fq -- "${repo_root}/diesel.toml" <<< "${compatible_output}"; then
    echo "Expected migrate_v2_compatible to invoke run_migration with diesel.toml"
    printf '%s\n' "${compatible_output}"
    exit 1
fi

if grep -Fq -- "just run_migration run ${repo_root}/final-migrations ${default_db_url}" <<< "${compatible_output}"; then
    echo "migrate_v2_compatible is still passing DATABASE_URL as the third positional argument"
    printf '%s\n' "${compatible_output}"
    exit 1
fi

v2_output="$(just --justfile "${repo_root}/justfile" --dry-run migrate_v2 run 2>&1)"

if ! grep -Fq -- "${repo_root}/diesel_v2.toml" <<< "${v2_output}"; then
    echo "Expected migrate_v2 to invoke run_migration with diesel_v2.toml"
    printf '%s\n' "${v2_output}"
    exit 1
fi
