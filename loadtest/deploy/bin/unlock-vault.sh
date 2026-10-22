#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
deploy_dir="$(cd -- "$script_dir/.." && pwd)"
key_dir="${GENERATED_KEY_DIR:-$deploy_dir/generated/keys}"
vault_url="${VAULT_URL:-http://127.0.0.1:3001}"
tenant_id="${VAULT_TENANT_ID:-public}"

curl -fsS "$vault_url/health" >/dev/null

post_custodian() {
  local path="$1"
  local body="${2:-}"
  local response status payload
  local args=(-sS -X POST "$vault_url$path" -H "x-tenant-id: $tenant_id")
  if [[ -n "$body" ]]; then
    args+=(-H 'content-type: application/json' --data "$body")
  fi
  response="$(curl "${args[@]}" -w $'\n%{http_code}')"
  status="${response##*$'\n'}"
  payload="${response%$'\n'*}"
  if [[ "$status" == 2* ]]; then
    return 0
  fi
  if [[ "$status" == "400" ]] && grep -q '"code":"TE_03"' <<<"$payload"; then
    echo "Vault custodian is already unlocked."
    return 2
  fi
  echo "Vault custodian request $path failed: status=$status body=$payload" >&2
  return 1
}

for number in 1 2; do
  key="$(tr -d '\r\n' <"$key_dir/vault_custodian_key${number}.hex")"
  if post_custodian "/custodian/key${number}" "{\"key\":\"$key\"}"; then
    :
  else
    status=$?
    [[ "$status" -eq 2 ]] && exit 0
    exit "$status"
  fi
done

if post_custodian "/custodian/decrypt"; then
  :
else
  status=$?
  [[ "$status" -eq 2 ]] && exit 0
  exit "$status"
fi
echo "Vault custodian unlocked for tenant $tenant_id."
