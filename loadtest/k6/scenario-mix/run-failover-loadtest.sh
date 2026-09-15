#!/usr/bin/env bash
# Single-command entry point for a failover/recovery drill: starts a flat,
# sustained scenario-mix load that does NOT stop on failures (no
# abortOnFail thresholds — see config.failover.template.json), so you can
# trigger a failover mid-run and let the traffic keep flowing through it.
#
# An independent watcher process runs alongside k6 and writes
# recovery_report.txt (first/last 5xx timestamps and the gap between them)
# as soon as the run ends, whether it finishes naturally or you kill it
# early once you've seen recovery.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

ENV_FILE="failover.env"
DASHBOARD_PORT=5665

if [ ! -f "$ENV_FILE" ]; then
  echo "error: $ENV_FILE not found next to this script" >&2
  exit 1
fi
set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a

if [[ "${API_KEY:-}" == REPLACE_WITH_* || "${PUBLISHABLE_KEY:-}" == REPLACE_WITH_* ]]; then
  echo "error: $ENV_FILE still has placeholder API_KEY/PUBLISHABLE_KEY — fill in real merchant credentials first" >&2
  exit 1
fi

install_k6() {
  command -v k6 >/dev/null 2>&1 && return
  echo "k6 not found — installing..."
  if command -v dnf >/dev/null 2>&1; then
    sudo dnf install -y https://dl.k6.io/rpm/repo.rpm 2>/dev/null || {
      curl -s https://dl.k6.io/key.gpg | sudo rpm --import -
      sudo tee /etc/yum.repos.d/k6.repo >/dev/null <<'REPO'
[k6]
name=k6
baseurl=https://dl.k6.io/rpm
enabled=1
gpgcheck=1
gpgkey=https://dl.k6.io/key.gpg
REPO
      sudo dnf install -y k6
    }
  elif command -v apt-get >/dev/null 2>&1; then
    sudo gpg -k
    sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg \
      --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69
    echo "deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main" \
      | sudo tee /etc/apt/sources.list.d/k6.list >/dev/null
    sudo apt-get update -y && sudo apt-get install -y k6
  else
    echo "error: no dnf/apt-get found — install k6 manually: https://k6.io/docs/get-started/installation/" >&2
    exit 1
  fi
}

install_envsubst() {
  command -v envsubst >/dev/null 2>&1 && return
  echo "envsubst not found — installing gettext..."
  if command -v dnf >/dev/null 2>&1; then
    sudo dnf install -y gettext
  elif command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update -y && sudo apt-get install -y gettext-base
  else
    echo "error: no dnf/apt-get found — install gettext manually" >&2
    exit 1
  fi
}

install_python3() {
  command -v python3 >/dev/null 2>&1 && return
  echo "python3 not found — installing (needed for the recovery-time report)..."
  if command -v dnf >/dev/null 2>&1; then
    sudo dnf install -y python3
  elif command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update -y && sudo apt-get install -y python3
  else
    echo "error: no dnf/apt-get found — install python3 manually" >&2
    exit 1
  fi
}

install_k6
install_envsubst
install_python3

RUN_NAME="run_$(date +%Y%m%d_%H%M%S)"
OUT_DIR="output/$RUN_NAME"
mkdir -p "$OUT_DIR"

envsubst < config.failover.template.json > "$OUT_DIR/config.json"

echo "=== scenario-mix failover/recovery drill ==="
echo "config:      $OUT_DIR/config.json"
echo "rate:        ${FLAT_RPS} rps flat for ${DURATION_SECONDS}s — does NOT stop on failures"
echo "mix:         guest ${GUEST_WEIGHT}% / modular cit_off_session ${MODULAR_CIT_OFF_WEIGHT}%"
echo "output dir:  $OUT_DIR"
echo

# Validate before committing to a (potentially long) background run.
k6 inspect -e SCENARIO_MIX_CONFIG="$OUT_DIR/config.json" scenario-mix.js >/dev/null

nohup env SUMMARY_OUTPUT=summary.json OUTPUT_DIR="$OUT_DIR" SCENARIO_MIX_CONFIG="$OUT_DIR/config.json" \
  k6 run -q \
  --console-output="$OUT_DIR/failures.log" \
  --out "web-dashboard=export=$OUT_DIR/timeseries.html&period=2s" \
  scenario-mix.js > "$OUT_DIR/run.log" 2>&1 &
K6_PID=$!
echo "$K6_PID" > "$OUT_DIR/k6.pid"

sleep 2
if ! kill -0 "$K6_PID" 2>/dev/null; then
  echo "error: k6 exited immediately — check $OUT_DIR/run.log" >&2
  tail -n 40 "$OUT_DIR/run.log" >&2
  exit 1
fi

# Independent watcher: polls for k6 to exit (naturally at DURATION_SECONDS,
# or because you killed it early) and then writes the recovery report.
# Deliberately a separate process from k6 itself, so `kill $(cat k6.pid)`
# keeps working exactly like the ramp runbook's.
nohup bash -c "
  while kill -0 $K6_PID 2>/dev/null; do sleep 5; done
  python3 analyze_recovery.py '$OUT_DIR/failures.log' > '$OUT_DIR/recovery_report.txt' 2>&1
" > "$OUT_DIR/watcher.log" 2>&1 &
disown -a

echo "k6 started (pid $K6_PID)"
echo "dashboard is live on this machine at: http://127.0.0.1:${DASHBOARD_PORT}"
echo "  -> port-forward it to view from your own machine, see REMOTE_RUN.md 'View the dashboard'"
echo "live progress:      tail -f $OUT_DIR/run.log"
echo "live failure count: wc -l $OUT_DIR/failures.log"
echo "live recovery check (safe to run any time, before or after the run ends):"
echo "                     python3 analyze_recovery.py $OUT_DIR/failures.log"
echo "auto-generated once the run ends (naturally or via kill): $OUT_DIR/recovery_report.txt"
echo "stop early:          kill \$(cat $OUT_DIR/k6.pid)"
