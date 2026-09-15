#!/usr/bin/env bash
# Single-command entry point for REMOTE_RUN.md: installs k6/envsubst if
# missing, renders config.template.json from loadtest.env, and starts the
# ramp in the background so this command returns immediately.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

ENV_FILE="loadtest.env"
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
  echo "error: $ENV_FILE still has placeholder API_KEY/PUBLISHABLE_KEY — fill in real merchant credentials first (see REMOTE_RUN.md step 3)" >&2
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

install_k6
install_envsubst

RUN_NAME="run_$(date +%Y%m%d_%H%M%S)"
OUT_DIR="output/$RUN_NAME"
mkdir -p "$OUT_DIR"

envsubst < config.template.json > "$OUT_DIR/config.json"

echo "=== scenario-mix remote run ==="
echo "config:      $OUT_DIR/config.json"
echo "ramp:        ${STARTING_RPS} -> ${TARGET_RPS} rps, step ${STEP_RPS}, hold ${HOLD_SECONDS}s, idle ${IDLE_SECONDS}s"
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

echo "k6 started (pid $K6_PID)"
echo "dashboard is live on this machine at: http://127.0.0.1:${DASHBOARD_PORT}"
echo "  -> port-forward it to view from your own machine, see REMOTE_RUN.md 'View the dashboard'"
echo "live progress:   tail -f $OUT_DIR/run.log"
echo "failure count:   wc -l $OUT_DIR/failures.log"
echo "stop early:      kill \$(cat $OUT_DIR/k6.pid)"
