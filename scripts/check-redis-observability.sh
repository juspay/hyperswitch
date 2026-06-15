#! /usr/bin/env bash
set -euo pipefail

# Guards against observability coverage drift in the Redis command layer.
#
# Decision: every Redis command emits exactly one `ExternalServiceCall` event,
# from a method-level `observed!` wrapper at the logical-operation boundary (the
# same pattern as keymanager / UCS). See
# crates/analytics/docs/redis_instrumentation_plan.md ("Design Decision").
#
# Invariant enforced here: any `pub async fn` in the command layer that performs
# its OWN Redis I/O (i.e. calls `track_redis_call`) must also be wrapped in
# `observed!`. Delegating wrappers (serialize_and_*, get_and_deserialize_*, ...)
# call already-instrumented leaves, contain no `track_redis_call`, and are
# therefore exempt — instrumenting them would double-count one logical operation.

files=(
  "crates/redis_interface/src/module/redis_rs/commands.rs"
  "crates/redis_interface/src/module/fred/commands.rs"
)

status=0
for file in "${files[@]}"; do
  missing="$(
    awk '
      function flush() {
        if (in_pub && body ~ /track_redis_call/ && body !~ /observed!/) {
          printf "  %s: %s\n", FILENAME, name
        }
      }
      /^    (pub )?async fn / || /^    pub fn / || /^}/ {
        flush()
        in_pub = ($0 ~ /^    pub async fn /)
        name = $0
        sub(/^[[:space:]]*(pub )?(async )?fn /, "", name)
        sub(/[<(].*/, "", name)
        body = ""
        next
      }
      { body = body "\n" $0 }
      END { flush() }
    ' "${file}"
  )"
  if [[ -n "${missing}" ]]; then
    if [[ "${status}" -eq 0 ]]; then
      echo "ERROR: Redis command methods do their own I/O (track_redis_call) but emit no"
      echo "       ExternalServiceCall event (missing observed!):"
    fi
    echo "${missing}"
    status=1
  fi
done

if [[ "${status}" -ne 0 ]]; then
  echo
  echo "Wrap the method body in: crate::observed!(self, \"CMD\", { ... })"
  echo "so each Redis operation emits exactly one event."
  exit 1
fi

echo "Redis observability coverage: OK (both backends)."
