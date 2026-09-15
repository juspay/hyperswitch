#!/bin/sh
# Start Jaeger, then seed it from the statically-hosted trace bundle.
#
# Render free spins the service down when idle and starts it with an empty
# filesystem, so the traces have to come from somewhere external on each boot.
# TRACE_MANIFEST points at a Netlify-hosted JSON list of trace URLs.
set -eu

/cmd/jaeger/jaeger-linux --config /etc/jaeger/config.yaml &
JAEGER_PID=$!

# Wait for the OTLP HTTP receiver before pushing anything at it.
i=0
while [ "$i" -lt 60 ]; do
  if wget -q -O /dev/null "http://127.0.0.1:16686/" 2>/dev/null; then break; fi
  i=$((i+1)); sleep 1
done

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
      wget -q -O /tmp/t.json "$url" && \
        wget -q -O /dev/null --header='Content-Type: application/json' \
          --post-file=/tmp/t.json "http://127.0.0.1:4318/v1/traces" && \
        echo "  ok" || echo "  FAILED $url"
    done
  fi
fi

wait "$JAEGER_PID"
