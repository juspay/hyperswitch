#!/bin/sh
# Start Jaeger, then seed it from the statically-hosted trace bundle.
#
# Render free spins the service down when idle and starts it with an empty
# filesystem, so the traces have to come from somewhere external on each boot.
# TRACE_MANIFEST points at a Netlify-hosted JSON list of trace URLs.
set -eu

QUERY_PORT="${PORT:-16686}"
OTLP_HTTP=4318

# The image's config is read-only and the query port is only known at runtime,
# so render the real config into /tmp.
sed "s/__QUERY_PORT__/${QUERY_PORT}/" /etc/jaeger/config.yaml > /tmp/config.yaml

/cmd/jaeger/jaeger-linux --config /tmp/config.yaml &
JAEGER_PID=$!

# Wait for the OTLP receiver specifically. The query port comes up first, so
# probing that (as this script used to) returns ready while /v1/traces is still
# refusing connections -- the seed then fails and Jaeger serves an empty UI.
i=0
while [ "$i" -lt 60 ]; do
  if wget -q -O /dev/null "http://127.0.0.1:${OTLP_HTTP}/v1/traces" 2>/dev/null; then break; fi
  # A listening receiver rejects that empty GET, which is still proof it is up;
  # only a refused connection means "not yet".
  if wget -S -O /dev/null "http://127.0.0.1:${OTLP_HTTP}/v1/traces" 2>&1 | grep -q "HTTP/"; then break; fi
  i=$((i+1)); sleep 1
done

seeded=0
if [ -n "${TRACE_MANIFEST:-}" ]; then
  echo "seeding traces from $TRACE_MANIFEST"
  wget -q -O /tmp/manifest.json "$TRACE_MANIFEST" || echo "manifest fetch failed; starting empty"
  if [ -s /tmp/manifest.json ]; then
    # manifest is a plain JSON array of URLs
    # entries may be relative ("/traces/x.json"); resolve them against the
    # manifest's own origin so the bundle stays portable across hostnames
    BASE=$(echo "$TRACE_MANIFEST" | sed -E 's#(https?://[^/]+).*#\1#')
    sed 's/[][]//g; s/"//g; s/,/\n/g' /tmp/manifest.json | while read -r url; do
      url=$(echo "$url" | tr -d ' ')
      [ -z "$url" ] && continue
      case "$url" in /*) url="${BASE}${url}" ;; esac
      echo "  loading $url"
      if ! wget -q -O /tmp/t.json "$url"; then
        echo "  FAILED to download $url"; continue
      fi
      # OTLP answers 200 with {"partialSuccess":{...}} even when it drops spans,
      # so the status code alone does not mean the traces are in.
      resp=$(wget -q -O - --header='Content-Type: application/json' \
               --post-file=/tmp/t.json "http://127.0.0.1:${OTLP_HTTP}/v1/traces" 2>/dev/null) || {
        echo "  FAILED to post $url"; continue
      }
      case "$resp" in
        *rejectedSpans*) echo "  PARTIAL $url -> $resp" ;;
        *)               echo "  ok $url" ;;
      esac
    done
    seeded=1
  fi
else
  echo "TRACE_MANIFEST is unset -- starting with an empty Jaeger"
fi

# Report what actually made it into storage. Without this a failed seed is
# indistinguishable from a healthy empty instance.
if [ "$seeded" = 1 ]; then
  ( sleep 5
    svc=$(wget -q -O - "http://127.0.0.1:${QUERY_PORT}/api/services" 2>/dev/null || echo '?')
    echo "seed result, services now in storage: $svc" ) &
fi

echo "jaeger query UI listening on ${QUERY_PORT}"
wait "$JAEGER_PID"
