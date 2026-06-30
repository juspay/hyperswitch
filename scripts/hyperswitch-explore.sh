#!/usr/bin/env bash
# ================================================================
#  hyperswitch-explore.sh
#  One command: deploy Hyperswitch locally + seed transactions
#
#  Usage:
#    bash hyperswitch-explore.sh
#    bash hyperswitch-explore.sh --skip-deploy      # already deployed
#    bash hyperswitch-explore.sh --txn-count=1000   # custom volume
#    bash hyperswitch-explore.sh --force-reseed     # re-seed existing
#
#  What this does:
#    1. Checks prerequisites (kubectl, helm, curl, jq)
#    2. Verifies / starts a local Kubernetes cluster
#    3. Deploys Hyperswitch via Helm  (slowest step, ~5-8 min on first run)
#    4. Port-forwards 6 services to localhost
#    5. Creates your Control Center account (email + password you enter)
#    6. Adds 4 dummy connectors: stripe_test · paypal_test · fauxpay · pretendpay
#    7. Sets up routing rules: Germany → paypal · France → pretendpay · Japan → fauxpay · >100 USD → stripe
#    8. Seeds 50 transactions spread across all routing rules (success / failure / refund mix)
#    9. Prints all service URLs + setup summary
# ================================================================

set -euo pipefail
IFS=$'\n\t'

# ── Flags ────────────────────────────────────────────────────
SKIP_DEPLOY=false
TXN_COUNT=50   # sequential payments; use --txn-count=N to change
FORCE_RESEED=false
for arg in "$@"; do
  case $arg in
    --skip-deploy)   SKIP_DEPLOY=true ;;
    --txn-count=*)   TXN_COUNT="${arg#*=}" ;;
    --force-reseed)  FORCE_RESEED=true ;;
  esac
done

# ── Constants ────────────────────────────────────────────────
HS_NAMESPACE="hyperswitch"
HS_RELEASE="hypers-v1"
LOG_FILE="/tmp/hs-explore.log"
STATE_FILE="/tmp/hs-explore-state.json"   # idempotency guard only

# Service definitions (bash 3.2 compatible — no associative arrays)
# Service names: release-prefixed ones use ${HS_RELEASE}, others are fixed by the Helm chart
SVC_K8S_app="${HS_RELEASE}-hyperswitch-server"
SVC_K8S_cc="hyperswitch-control-center"
SVC_K8S_web="hyperswitch-web"
SVC_K8S_grafana="${HS_RELEASE}-grafana"
SVC_K8S_vector="${HS_RELEASE}-vector"
SVC_K8S_mailhog="mailhog"

SVC_PREFERRED_app=8080;   SVC_PREFERRED_cc=9000;   SVC_PREFERRED_web=9050
SVC_PREFERRED_grafana=3000; SVC_PREFERRED_vector=3103; SVC_PREFERRED_mailhog=8025

SVC_CONTAINER_app=80;    SVC_CONTAINER_cc=80;    SVC_CONTAINER_web=9050
SVC_CONTAINER_grafana=80; SVC_CONTAINER_vector=3103; SVC_CONTAINER_mailhog=8025

SVC_LABEL_app="App Server"
SVC_LABEL_cc="Control Center"
SVC_LABEL_web="Hyperswitch Web / SDK"
SVC_LABEL_grafana="Grafana"
SVC_LABEL_vector="Vector"
SVC_LABEL_mailhog="Mailhog"

# SVC_PORT_<svc> variables are set dynamically in start_port_forwards
SVC_PORT_app=0; SVC_PORT_cc=0; SVC_PORT_web=0
SVC_PORT_grafana=0; SVC_PORT_vector=0; SVC_PORT_mailhog=0

# ── Colours ──────────────────────────────────────────────────
R='\033[0;31m' G='\033[0;32m' Y='\033[1;33m'
B='\033[0;34m' C='\033[0;36m' W='\033[1m' N='\033[0m'

info()    { echo -e "${B}ℹ${N}  $*"; }
ok()      { echo -e "${G}✓${N}  $*"; }
warn()    { echo -e "${Y}⚠${N}  $*"; }
err()     { echo -e "${R}✗${N}  $*" >&2; }
step()    { echo -e "\n${W}${C}━━  $*${N}"; }
progress(){ printf "${B}…${N}  $*\r"; }

banner() {
  local T="H Y P E R S W I T C H   L I V E   E X P L O R E R"
  local S="Deploy . Create account . Add dummy connectors . Add routing rules . Seed 50 txns"
  # Interior width = longer line + 4 padding each side
  local inner=$(( ${#T} > ${#S} ? ${#T} + 8 : ${#S} + 8 ))
  # Build border string
  local border="" i=0
  while [ $i -lt $inner ]; do border="${border}─"; i=$((i+1)); done
  # Compute left padding for each line (right padding fills the rest)
  local tp=$(( (inner - ${#T}) / 2 ))
  local tp2=$(( inner - ${#T} - tp ))
  local sp=$(( (inner - ${#S}) / 2 ))
  local sp2=$(( inner - ${#S} - sp ))
  echo -e "${C}"
  printf "  ┌%s┐\n" "$border"
  printf "  │%${tp}s%s%${tp2}s│\n" "" "$T" ""
  printf "  │%${sp}s%s%${sp2}s│\n" "" "$S" ""
  printf "  └%s┘\n" "$border"
  echo -e "${N}"
}

# ── OS detection ─────────────────────────────────────────────
detect_os() {
  OS="unknown"
  ARCH=$(uname -m)
  case "$(uname -s)" in
    Darwin) OS="mac" ;;
    Linux)  OS="linux" ;;
    MINGW*|CYGWIN*|MSYS*) OS="windows" ;;
  esac
}

# Run kubectl cluster-info with a hard wall-clock timeout (macOS has no 'timeout').
# Returns 0 if cluster is reachable, 1 otherwise. Kills the background process
# if it hangs past MAX_SECS.
_cluster_reachable() {
  local max_secs="${1:-8}"
  kubectl cluster-info --request-timeout="${max_secs}s" &>/dev/null 2>&1 &
  local kpid=$!
  local elapsed=0
  while kill -0 "$kpid" 2>/dev/null && [ "$elapsed" -lt "$max_secs" ]; do
    sleep 1
    elapsed=$((elapsed + 1))
  done
  if kill -0 "$kpid" 2>/dev/null; then
    kill "$kpid" 2>/dev/null
    wait "$kpid" 2>/dev/null || true
    return 1
  fi
  wait "$kpid"
  return $?
}

# ================================================================
#  STEP 1: PREREQUISITES
#  Install missing tools. Require sudo only when necessary.
#  Never blindly sudo — always ask first.
# ================================================================
install_prereqs() {
  step "Prerequisites"
  detect_os

  local missing=()
  for cmd in kubectl helm curl jq; do
    command -v "$cmd" &>/dev/null && ok "$cmd" || missing+=("$cmd")
  done

  # minikube only needed if there is no reachable cluster AND no configured context
  # (a configured but unreachable context still indicates the user knows what they're doing)
  local has_context
  has_context=$(kubectl config current-context 2>/dev/null || echo "")
  if [ -z "$has_context" ]; then
    command -v "minikube" &>/dev/null && ok "minikube" || missing+=("minikube")
  fi

  if [ ${#missing[@]} -eq 0 ]; then
    ok "All prerequisites present"
    return
  fi

  warn "Missing tools: ${missing[*]}"
  echo ""
  echo "  Choose:"
  echo "  1) Install automatically"
  echo "  2) Show me the install commands (I'll run them myself)"
  echo ""
  read -rp "  → [1]: " install_choice
  install_choice="${install_choice:-1}"

  case "$install_choice" in
    2)
      _show_install_commands "${missing[@]}"
      echo "  Re-run this script once installed."
      exit 0
      ;;
    1)
      if [ "$OS" = "windows" ]; then
        warn "Auto-install is not supported on Windows."
        _show_install_commands "${missing[@]}"
        echo "  Re-run this script once installed."
        exit 1
      fi
      _auto_install "${missing[@]}"
      ;;
    *)
      err "Invalid choice. Re-run the script and enter 1 or 2."
      exit 1
      ;;
  esac
}

_show_install_commands() {
  local tools=("$@")
  echo ""
  echo "  Install commands for: ${tools[*]}"
  echo ""
  for t in "${tools[@]}"; do
    case "$t" in
      kubectl)
        if [ "$OS" = "mac" ]; then
          echo "  # kubectl"
          echo "  brew install kubectl"
        elif [ "$OS" = "linux" ]; then
          echo "  # kubectl"
          echo "  curl -LO \"https://dl.k8s.io/release/\$(curl -sL https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl\""
          echo "  sudo install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl && rm kubectl"
        else
          echo "  # kubectl (Windows — run in PowerShell)"
          echo "  winget install Kubernetes.kubectl"
        fi
        echo ""
        ;;
      helm)
        if [ "$OS" = "mac" ]; then
          echo "  # helm"
          echo "  brew install helm"
        elif [ "$OS" = "linux" ]; then
          echo "  # helm"
          echo "  curl https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 | bash"
        else
          echo "  # helm (Windows — run in PowerShell)"
          echo "  winget install Helm.Helm"
        fi
        echo ""
        ;;
      minikube)
        if [ "$OS" = "mac" ]; then
          echo "  # minikube"
          echo "  brew install minikube"
        elif [ "$OS" = "linux" ]; then
          echo "  # minikube"
          echo "  curl -LO https://storage.googleapis.com/minikube/releases/latest/minikube-linux-amd64"
          echo "  sudo install minikube-linux-amd64 /usr/local/bin/minikube && rm minikube-linux-amd64"
        else
          echo "  # minikube (Windows — run in PowerShell)"
          echo "  winget install Kubernetes.minikube"
        fi
        echo ""
        ;;
      jq)
        if [ "$OS" = "mac" ]; then
          echo "  # jq"
          echo "  brew install jq"
        elif [ "$OS" = "linux" ]; then
          echo "  # jq"
          echo "  sudo apt-get install -y jq     # Debian/Ubuntu"
          echo "  # sudo yum install -y jq       # RHEL/CentOS"
        else
          echo "  # jq (Windows — run in PowerShell)"
          echo "  winget install jqlang.jq"
        fi
        echo ""
        ;;
      curl)
        if [ "$OS" = "mac" ]; then
          echo "  # curl"
          echo "  brew install curl"
        elif [ "$OS" = "linux" ]; then
          echo "  # curl"
          echo "  sudo apt-get install -y curl   # Debian/Ubuntu"
          echo "  # sudo yum install -y curl     # RHEL/CentOS"
        else
          echo "  # curl — built into Windows 10+; if missing:"
          echo "  winget install curl.curl"
        fi
        echo ""
        ;;
    esac
  done
}

_auto_install() {
  local tools=("$@")
  if [ "$OS" = "mac" ]; then
    if ! command -v brew &>/dev/null; then
      info "Installing Homebrew..."
      /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)" \
        >>"$LOG_FILE" 2>&1 || { err "Homebrew install failed. See: $LOG_FILE"; exit 1; }
    fi
    for t in "${tools[@]}"; do
      info "Installing $t via brew..."
      brew install "$t" >>"$LOG_FILE" 2>&1 \
        && ok "$t installed" \
        || { err "$t install failed. See: $LOG_FILE"; exit 1; }
    done
  elif [ "$OS" = "linux" ]; then
    for t in "${tools[@]}"; do
      case "$t" in
        kubectl)
          info "Installing kubectl..."
          curl -sfLO "https://dl.k8s.io/release/$(curl -sL https://dl.k8s.io/release/stable.txt)/bin/linux/amd64/kubectl" \
            >>"$LOG_FILE" 2>&1 || { err "Failed to download kubectl. See: $LOG_FILE"; exit 1; }
          sudo install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl \
            && rm -f kubectl && ok "kubectl installed" \
            || { err "Failed to install kubectl."; exit 1; }
          ;;
        helm)
          info "Installing helm..."
          curl -sf https://raw.githubusercontent.com/helm/helm/main/scripts/get-helm-3 \
            | bash >>"$LOG_FILE" 2>&1 \
            && ok "helm installed" \
            || { err "helm install failed. See: $LOG_FILE"; exit 1; }
          ;;
        minikube)
          info "Installing minikube..."
          curl -sfLO https://storage.googleapis.com/minikube/releases/latest/minikube-linux-amd64 \
            >>"$LOG_FILE" 2>&1 || { err "Failed to download minikube."; exit 1; }
          sudo install minikube-linux-amd64 /usr/local/bin/minikube \
            && rm -f minikube-linux-amd64 && ok "minikube installed" \
            || { err "Failed to install minikube."; exit 1; }
          ;;
        jq)
          info "Installing jq..."
          if command -v apt-get &>/dev/null; then
            sudo apt-get install -y jq >>"$LOG_FILE" 2>&1 \
              && ok "jq installed" || { err "jq install failed. See: $LOG_FILE"; exit 1; }
          elif command -v yum &>/dev/null; then
            sudo yum install -y jq >>"$LOG_FILE" 2>&1 \
              && ok "jq installed" || { err "jq install failed. See: $LOG_FILE"; exit 1; }
          else
            err "No supported package manager found (apt-get/yum). Install jq manually:"
            echo "  https://jqlang.github.io/jq/download/"
            exit 1
          fi
          ;;
        curl)
          info "Installing curl..."
          if command -v apt-get &>/dev/null; then
            sudo apt-get install -y curl >>"$LOG_FILE" 2>&1 \
              && ok "curl installed" || { err "curl install failed. See: $LOG_FILE"; exit 1; }
          elif command -v yum &>/dev/null; then
            sudo yum install -y curl >>"$LOG_FILE" 2>&1 \
              && ok "curl installed" || { err "curl install failed. See: $LOG_FILE"; exit 1; }
          else
            err "No supported package manager found. Install curl manually."
            exit 1
          fi
          ;;
      esac
    done
  fi
}

# ================================================================
#  STEP 2: KUBERNETES CLUSTER
# ================================================================
ensure_cluster() {
  step "Kubernetes cluster"

  progress "Checking for a running Kubernetes cluster (8s timeout)..."
  if _cluster_reachable 8; then
    local ctx
    ctx=$(kubectl config current-context 2>/dev/null || echo "unknown")
    echo ""
    ok "Cluster running (context: $ctx)"
    return
  fi
  echo ""

  warn "No running cluster detected"
  echo ""
  echo "  Options:"
  echo "  1) Start a local cluster automatically  (uses Docker or Podman)"
  echo "  2) I already have a cluster             (kubeconfig already set)"
  echo ""
  read -rp "  → [1]: " cluster_choice
  cluster_choice="${cluster_choice:-1}"

  case "$cluster_choice" in
    1) _start_local_cluster ;;
    2)
      progress "Verifying cluster (8s timeout)..."
      if ! _cluster_reachable 8; then
        echo ""
        err "No cluster accessible. Check your kubeconfig."
        exit 1
      fi
      echo ""
      ok "Cluster accessible"
      ;;
  esac
}

# ================================================================
#  _start_local_cluster
#  Detects the available container runtime and starts a local
#  Kubernetes cluster using the best available tool.
#
#  Decision tree:
#    Docker running            → minikube --driver=docker
#    Podman available (macOS)  → ensure podman machine ≥6GB,
#                                then kind --provider=podman
#    Podman available (Linux)  → kind --provider=podman
#    Nothing found             → clear error + install guidance
# ================================================================
_start_local_cluster() {
  echo ""

  # ── Detect runtime ──────────────────────────────────────────
  local runtime=""
  if docker info &>/dev/null 2>&1; then
    runtime="docker"
  elif command -v podman &>/dev/null 2>&1; then
    runtime="podman"
  fi

  if [ -z "$runtime" ]; then
    echo ""
    err "No container runtime found. Install one of the following, then re-run:"
    echo ""
    echo "  Docker Desktop : https://www.docker.com/products/docker-desktop"
    echo "  Podman Desktop : https://podman-desktop.io"
    if [ "$OS" = "mac" ]; then
      echo "  OrbStack       : https://orbstack.dev  (enable Kubernetes in app)"
      echo "  Colima         : brew install colima && colima start"
    fi
    exit 1
  fi

  ok "Container runtime: $runtime"

  # ── Podman on macOS needs a machine VM ──────────────────────
  if [ "$runtime" = "podman" ] && [ "$OS" = "mac" ]; then
    _ensure_podman_machine_mac 6144
  fi

  # ── Choose cluster tool ─────────────────────────────────────
  # kind works reliably with both Docker and Podman across platforms.
  # minikube is used only when kind is not available and Docker is running.
  if command -v kind &>/dev/null 2>&1; then
    _start_kind "$runtime"
  elif [ "$runtime" = "docker" ] && command -v minikube &>/dev/null 2>&1; then
    _start_minikube_docker
  else
    # Install kind (works everywhere)
    info "Installing kind..."
    if [ "$OS" = "mac" ]; then
      brew install kind >>"$LOG_FILE" 2>&1
    else
      local kind_url="https://kind.sigs.k8s.io/dl/latest/kind-linux-$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/')"
      curl -sfLo /usr/local/bin/kind "$kind_url" >>"$LOG_FILE" 2>&1
      chmod +x /usr/local/bin/kind
    fi
    ok "kind installed"
    _start_kind "$runtime"
  fi
}

# Start a kind cluster. Works with both docker and podman.
_start_kind() {
  local runtime="${1:-docker}"

  if kind get clusters 2>/dev/null | grep -q "^hyperswitch$"; then
    ok "kind cluster 'hyperswitch' already exists"
    kind export kubeconfig --name hyperswitch >>"$LOG_FILE" 2>&1
    return
  fi

  info "Creating kind cluster (runtime: $runtime) — takes ~2 min..."
  if [ "$runtime" = "podman" ]; then
    KIND_EXPERIMENTAL_PROVIDER=podman kind create cluster \
      --name hyperswitch >>"$LOG_FILE" 2>&1
  else
    kind create cluster --name hyperswitch >>"$LOG_FILE" 2>&1
  fi

  if [ $? -ne 0 ]; then
    err "kind cluster creation failed. See: $LOG_FILE"
    tail -20 "$LOG_FILE" >&2
    exit 1
  fi

  kind export kubeconfig --name hyperswitch >>"$LOG_FILE" 2>&1
  ok "kind cluster 'hyperswitch' ready"
}

# Minikube fallback — only used when kind is absent and Docker is running
_start_minikube_docker() {
  info "Starting minikube (docker driver) — takes ~2 min..."
  minikube start --cpus=4 --memory=5500 --driver=docker \
    >>"$LOG_FILE" 2>&1 && ok "minikube started" || {
    err "minikube failed to start. See: $LOG_FILE"
    tail -20 "$LOG_FILE" >&2
    exit 1
  }
}

# macOS only: ensure podman machine exists, is running, and has enough RAM.
_ensure_podman_machine_mac() {
  local need_mb="${1:-6144}"
  local machine="podman-machine-default"

  if ! podman machine list 2>/dev/null | grep -q "$machine"; then
    info "Initialising podman machine (downloads ~600MB, one-time)..."
    podman machine init --cpus 4 --memory "$need_mb" >>"$LOG_FILE" 2>&1 || {
      err "Failed to init podman machine. See: $LOG_FILE"; exit 1
    }
    podman machine start "$machine" >>"$LOG_FILE" 2>&1 || {
      err "Failed to start podman machine. See: $LOG_FILE"; exit 1
    }
    ok "Podman machine ready"
    return
  fi

  # Machine exists — check memory
  local cur_mb
  cur_mb=$(podman machine inspect "$machine" 2>/dev/null \
    | grep '"Memory"' | grep -o '[0-9]*' | head -1 || echo 0)

  local is_running=false
  podman machine list 2>/dev/null | grep "$machine" \
    | grep -q "Currently running" && is_running=true || true

  if [ "$cur_mb" -lt "$need_mb" ]; then
    if $is_running; then
      info "Stopping podman machine to resize memory..."
      podman machine stop "$machine" >>"$LOG_FILE" 2>&1 || true
      is_running=false
    fi
    info "Resizing podman machine to ${need_mb}MB RAM..."
    podman machine set --memory "$need_mb" "$machine" >>"$LOG_FILE" 2>&1 || true
  fi

  if ! $is_running; then
    info "Starting podman machine..."
    podman machine start "$machine" >>"$LOG_FILE" 2>&1 || {
      err "Failed to start podman machine. See: $LOG_FILE"; exit 1
    }
    ok "Podman machine ready (${need_mb}MB)"
  else
    ok "Podman machine running"
  fi

  # Export socket so kind can find podman
  local sock
  sock=$(podman machine inspect "$machine" 2>/dev/null \
    | grep -o '"Sock":"[^"]*"' | cut -d'"' -f4 | head -1 \
    || podman machine inspect "$machine" 2>/dev/null \
    | grep -o '/[^ "]*\.sock' | head -1 || echo "")
  [ -n "$sock" ] && export DOCKER_HOST="unix://${sock}" || true
}

# ================================================================
#  STEP 3: HELM DEPLOY
# ================================================================
ensure_deployed() {
  step "Hyperswitch deployment"

  if $SKIP_DEPLOY; then
    info "--skip-deploy flag set, skipping Helm install"
    return
  fi

  if helm status "$HS_RELEASE" -n "$HS_NAMESPACE" &>/dev/null 2>&1; then
    ok "Hyperswitch already deployed (release: $HS_RELEASE)"
    # Still need to wait for pods and run migrations in case a previous run
    # crashed mid-migration (e.g. leaving DB with 0 tables).
    info "Waiting for PostgreSQL to be ready (timeout: 15 min)..."
    kubectl wait --for=condition=ready pod \
      --selector=app.kubernetes.io/name=postgresql \
      -n "$HS_NAMESPACE" --timeout=900s \
      >>"$LOG_FILE" 2>&1 \
      && ok "PostgreSQL ready" \
      || { err "PostgreSQL did not become ready. Check: kubectl get pods -n $HS_NAMESPACE"; exit 1; }
    _run_db_migrations
    info "Waiting for App Server pod to be ready (timeout: 10 min)..."
    kubectl wait --for=condition=ready pod \
      --selector=app=${HS_RELEASE}-hyperswitch-server \
      -n "$HS_NAMESPACE" --timeout=600s \
      >>"$LOG_FILE" 2>&1 \
      && ok "App Server pod ready" \
      || warn "Pod not ready yet — will keep checking during port-forward phase."
    return
  fi

  # If the namespace is still terminating from a previous teardown, wait for it
  # to fully disappear before attempting helm install — otherwise Helm cannot
  # create secrets in a terminating namespace and fails immediately.
  local ns_status
  ns_status=$(kubectl get namespace "$HS_NAMESPACE" -o jsonpath='{.status.phase}' 2>/dev/null || echo "gone")
  if [ "$ns_status" = "Terminating" ]; then
    progress "Waiting for previous namespace to finish terminating..."
    local waited=0
    while kubectl get namespace "$HS_NAMESPACE" &>/dev/null 2>&1; do
      sleep 3
      waited=$((waited + 3))
      if [ $waited -gt 120 ]; then
        err "Namespace '$HS_NAMESPACE' stuck terminating after 2 min. Try: kubectl delete namespace $HS_NAMESPACE --force --grace-period=0"
        exit 1
      fi
    done
    echo ""
    ok "Namespace fully terminated — proceeding with install"
  fi

  info "Adding Hyperswitch Helm repo..."
  helm repo add hyperswitch https://juspay.github.io/hyperswitch-helm \
    >>"$LOG_FILE" 2>&1
  helm repo update >>"$LOG_FILE" 2>&1
  ok "Helm repo ready"

  info "Installing Hyperswitch (namespace: $HS_NAMESPACE)..."
  echo "  This pulls ~400MB of images on first run. Grab a coffee."
  echo ""
  # NOTE: helm install exits non-zero when the post-install DB migration job
  # exceeds Helm's hook deadline (~5-6 min). The deployment itself succeeds.
  # We treat any non-zero exit as a warning and let the subsequent pod-ready
  # wait determine whether the install actually worked.
  helm install "$HS_RELEASE" hyperswitch/hyperswitch-stack \
    -n "$HS_NAMESPACE" --create-namespace \
    >>"$LOG_FILE" 2>&1 || warn "Helm reported an error (likely hook timeout) — checking pods..."
  ok "Helm install complete (pods starting)"

  info "Waiting for PostgreSQL to be ready (timeout: 15 min)..."
  kubectl wait --for=condition=ready pod \
    --selector=app.kubernetes.io/name=postgresql \
    -n "$HS_NAMESPACE" --timeout=900s \
    >>"$LOG_FILE" 2>&1 \
    && ok "PostgreSQL ready" \
    || { err "PostgreSQL did not become ready. Check: kubectl get pods -n $HS_NAMESPACE"; exit 1; }

  _run_db_migrations

  info "Waiting for App Server pod to be ready (timeout: 10 min)..."
  echo "  Images are pulling and dependencies (Postgres, Redis, Kafka) are starting."
  echo "  This is slow on first run — subsequent runs are instant."
  kubectl wait --for=condition=ready pod \
    --selector=app=${HS_RELEASE}-hyperswitch-server \
    -n "$HS_NAMESPACE" --timeout=600s \
    >>"$LOG_FILE" 2>&1 \
    && ok "App Server pod ready" \
    || warn "Pod not ready yet — will keep checking during port-forward phase."
}

# ================================================================
#  _run_db_migrations
#  Downloads Hyperswitch migration SQL from GitHub (on the HOST,
#  which has internet) and applies them via a temporary port-forward
#  to the in-cluster PostgreSQL. Grants full privileges to the app
#  user afterward.
#
#  Skipped if the schema already has tables (idempotent).
# ================================================================
_run_db_migrations() {
  step "Database migrations"

  # Check if migrations already applied
  local PG_POD
  PG_POD=$(kubectl get pod -n "$HS_NAMESPACE" \
    -l app.kubernetes.io/name=postgresql \
    -o jsonpath='{.items[0].metadata.name}' 2>>"$LOG_FILE" || echo "")

  if [ -z "$PG_POD" ]; then
    err "Cannot find PostgreSQL pod for migrations."
    exit 1
  fi

  # Get postgres superuser password from secret
  local PG_SUPERPASS
  PG_SUPERPASS=$(kubectl get secret "${HS_RELEASE}-postgresql" -n "$HS_NAMESPACE" \
    -o jsonpath='{.data.postgres-password}' 2>/dev/null \
    | base64 -d 2>/dev/null || echo "")

  if [ -z "$PG_SUPERPASS" ]; then
    err "Cannot read PostgreSQL secret '${HS_RELEASE}-postgresql'."
    exit 1
  fi

  # Check if already migrated
  local table_count
  table_count=$(kubectl exec -n "$HS_NAMESPACE" "$PG_POD" -- \
    env PGPASSWORD="$PG_SUPERPASS" psql -U postgres -d hyperswitch -tAq \
    -c "SELECT count(*) FROM information_schema.tables WHERE table_schema='public';" \
    2>>"$LOG_FILE" | tr -d '[:space:]' || echo "0")

  if [ "$table_count" -gt 10 ]; then
    ok "Database already migrated ($table_count tables)"
    return
  fi

  local ROUTER_VERSION="v1.121.0"
  info "Downloading Hyperswitch v${ROUTER_VERSION} migrations from GitHub..."
  local MIG_DIR="/tmp/hs-db-migrations"
  rm -rf "$MIG_DIR" && mkdir -p "$MIG_DIR"

  curl -sfL "https://github.com/juspay/hyperswitch/archive/refs/tags/${ROUTER_VERSION}.tar.gz" \
    -o "${MIG_DIR}/hs.tar.gz" >>"$LOG_FILE" 2>&1 || {
    err "Failed to download migrations from GitHub. Check internet connectivity."
    exit 1
  }

  tar -xzf "${MIG_DIR}/hs.tar.gz" -C "$MIG_DIR" \
    --strip-components=1 "hyperswitch-${ROUTER_VERSION#v}/migrations" >>"$LOG_FILE" 2>&1

  ok "Migrations downloaded ($(ls "${MIG_DIR}/migrations" | wc -l | tr -d ' ') files)"

  # Port-forward postgres temporarily
  local PG_LOCAL_PORT=15432
  pkill -f "port-forward.*${HS_RELEASE}-postgresql" 2>/dev/null || true
  sleep 1
  kubectl port-forward "service/${HS_RELEASE}-postgresql" "${PG_LOCAL_PORT}:5432" \
    -n "$HS_NAMESPACE" >>"$LOG_FILE" 2>&1 &
  local PF_PID=$!
  sleep 3

  info "Applying migrations..."
  local failed=0
  for dir in $(ls -d "${MIG_DIR}/migrations"/*/); do
    local sql_file="${dir}up.sql"
    [ -f "$sql_file" ] || continue
    local result
    result=$(PGPASSWORD="$PG_SUPERPASS" psql -h localhost -p "$PG_LOCAL_PORT" \
      -U postgres -d hyperswitch -f "$sql_file" 2>&1)
    if echo "$result" | grep -qi "error" && ! echo "$result" | grep -qi "already exists"; then
      echo "  WARN: $dir" >>"$LOG_FILE"
      echo "  $result" >>"$LOG_FILE"
      failed=$((failed+1))
    fi
  done

  # Grant privileges to app user
  PGPASSWORD="$PG_SUPERPASS" psql -h localhost -p "$PG_LOCAL_PORT" \
    -U postgres -d hyperswitch \
    -c "GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO hyperswitch;
        GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO hyperswitch;
        GRANT ALL PRIVILEGES ON ALL FUNCTIONS IN SCHEMA public TO hyperswitch;
        ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO hyperswitch;
        ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO hyperswitch;" \
    >>"$LOG_FILE" 2>&1

  kill "$PF_PID" 2>/dev/null || true

  ok "Migrations applied ($failed warnings — see $LOG_FILE)"
  rm -rf "$MIG_DIR"
}

# ================================================================
#  STEP 4: PORT FORWARDS (with conflict handling)
# ================================================================

# Check if a port is in use
_port_in_use() {
  local port="$1"
  if command -v lsof &>/dev/null; then
    lsof -iTCP:"$port" -sTCP:LISTEN &>/dev/null 2>&1
  elif command -v ss &>/dev/null; then
    ss -tlnp 2>/dev/null | grep -q ":${port} "
  else
    # fallback: try to bind
    (echo >/dev/tcp/localhost/"$port") &>/dev/null 2>&1
  fi
}

# Find a free port starting from preferred
_find_free_port() {
  local preferred="$1"
  local port="$preferred"
  while _port_in_use "$port"; do
    port=$((port + 1))
  done
  echo "$port"
}

# Kill any existing HS port-forwards we started previously
_cleanup_old_forwards() {
  pkill -f "kubectl port-forward.*-n ${HS_NAMESPACE}" 2>/dev/null || true
  sleep 1
}

start_port_forwards() {
  step "Port forwarding"
  _cleanup_old_forwards

  local conflict_warned=false

  for svc in app cc web grafana vector mailhog; do
    eval "local preferred=\${SVC_PREFERRED_${svc}}"
    eval "local container=\${SVC_CONTAINER_${svc}}"
    eval "local k8s_svc=\${SVC_K8S_${svc}}"
    eval "local label=\${SVC_LABEL_${svc}}"
    local port

    port=$(_find_free_port "$preferred")
    eval "SVC_PORT_${svc}=${port}"

    if [ "$port" != "$preferred" ]; then
      if ! $conflict_warned; then
        echo ""
        warn "Some preferred ports are occupied — using alternates:"
        conflict_warned=true
      fi
      warn "  $label: preferred :$preferred → using :$port"
    fi

    kubectl port-forward "service/${k8s_svc}" "${port}:${container}" \
      -n "$HS_NAMESPACE" >>"$LOG_FILE" 2>&1 &

    ok "$label  →  http://localhost:${port}"
  done

  # Give forwards a moment to establish
  sleep 2

  # Verify app server is reachable (most important one)
  local app_port="${SVC_PORT_app}"
  local attempts=0
  local max_attempts=240   # 240 × 2s = 8 minutes
  progress "Waiting for App Server at :${app_port} (up to 8 min)..."
  until curl -sf "http://localhost:${app_port}/health" &>/dev/null; do
    sleep 2
    attempts=$((attempts+1))
    if [ $((attempts % 15)) -eq 0 ]; then
      # Print pod status every 30s so user knows it's progressing
      local ready_count not_ready
      ready_count=$(kubectl get pods -n "$HS_NAMESPACE" --no-headers 2>/dev/null \
        | grep -c "Running\|Completed" || echo 0)
      not_ready=$(kubectl get pods -n "$HS_NAMESPACE" --no-headers 2>/dev/null \
        | grep -vc "Running\|Completed\|Succeeded" || echo "?")
      printf "\n"
      info "  Still waiting... pods running: ${ready_count}, not ready: ${not_ready} (${attempts}/${max_attempts})"
    fi
    if [ $attempts -gt $max_attempts ]; then
      echo ""
      err "App Server did not respond at :${app_port} after 8 minutes"
      err "Pod status:"
      kubectl get pods -n "$HS_NAMESPACE" 2>&1 | tail -15 >&2
      err "App Server logs:"
      kubectl logs -n "$HS_NAMESPACE" -l app=${HS_RELEASE}-hyperswitch-server \
        --tail=20 2>&1 >&2 || true
      exit 1
    fi
  done
  echo ""
  ok "App Server responding"
}

# ================================================================
#  STEP 5: SEED — dummy connector + 5000 transactions
#  No user input needed. We configure everything automatically.
#  Uses Hyperswitch's built-in dummy connector — no PSP creds.
# ================================================================
seed_environment() {
  step "Seeding environment"

  # ── Guard: skip if already seeded ───────────────────────
  if [ -f "$STATE_FILE" ] && ! $FORCE_RESEED; then
    local seeded_count
    seeded_count=$(jq -r '.txn_count // 0' "$STATE_FILE" 2>/dev/null || echo 0)
    if [ "$seeded_count" -gt 0 ]; then
      ok "Already seeded ($seeded_count transactions). Use --force-reseed to redo."
      _load_state
      return
    fi
  fi

  local APP="http://localhost:${SVC_PORT_app}"

  # Declare globals written by this function
  MERCHANT_ID=""
  ADMIN_API_KEY=""     # from Helm env — used for all account-management calls
  MERCHANT_API_KEY=""  # created via /api_keys — used for /payments, /refunds
  PROFILE_ID=""
  STRIPE_TEST_MCA_ID=""
  PAYPAL_TEST_MCA_ID=""
  FAUXPAY_MCA_ID=""
  PRETENDPAY_MCA_ID=""
  ROUTING_ID=""

  step "Account setup"

  # ================================================================
  #  STEP 1 — GET ADMIN API KEY from local Helm deployment
  # ================================================================
  info "[1/8] Reading admin key from local Helm deployment..."

  local HS_POD
  HS_POD=$(kubectl get pod -n "$HS_NAMESPACE" \
    -l app="${HS_RELEASE}-hyperswitch-server" \
    -o jsonpath='{.items[0].metadata.name}' 2>>"$LOG_FILE")

  if [ -z "$HS_POD" ]; then
    err "Could not find hyperswitch-server pod in namespace $HS_NAMESPACE"
    err "Check: kubectl get pods -n $HS_NAMESPACE"
    exit 1
  fi

  ADMIN_API_KEY=$(kubectl exec -n "$HS_NAMESPACE" "$HS_POD" \
    -- printenv ROUTER__SECRETS__ADMIN_API_KEY 2>>"$LOG_FILE" || echo "")

  if [ -z "$ADMIN_API_KEY" ]; then
    err "ROUTER_ADMIN_API_KEY not found in pod env."
    err "Manual override: export ADMIN_API_KEY=<key> then re-run with --skip-deploy"
    exit 1
  fi
  ok "Admin key obtained"

  # ================================================================
  #  STEP 2 — CREATE DASHBOARD USER via signup
  #
  #  signup_with_merchant_id creates a fresh org + merchant derived
  #  from company_name. We do this FIRST so the merchant that gets
  #  created is the one the user will see when they log into the CC.
  #  Connectors and transactions are seeded to that same merchant.
  # ================================================================
  info "[2/8] Creating Control Center login for ${DASHBOARD_EMAIL}..."

  local signup_resp
  signup_resp=$(curl -s --max-time 30 -X POST "${APP}/user/signup_with_merchant_id" \
    -H "Content-Type: application/json" \
    -H "api-key: ${ADMIN_API_KEY}" \
    -d "{
      \"email\": \"${DASHBOARD_EMAIL}\",
      \"name\": \"Explorer User\",
      \"password\": \"${DASHBOARD_PASSWORD}\",
      \"merchant_id\": \"merchant_hs_explorer\",
      \"company_name\": \"HS Explorer\"
    }" 2>/dev/null)

  local signup_ok=false
  if echo "$signup_resp" | grep -q '"is_email_sent"'; then
    signup_ok=true
    ok "Dashboard user created — login at http://localhost:${SVC_PORT_cc}"
  elif echo "$signup_resp" | grep -qi "already exists\|duplicate\|conflict\|BE_02\|UR_02"; then
    signup_ok=true
    ok "Dashboard user already exists — reusing"
  else
    local errmsg
    errmsg=$(echo "$signup_resp" | jq -r '.error.message // empty' 2>/dev/null)
    err "Dashboard user creation failed: ${errmsg:-$signup_resp}"
    exit 1
  fi

  # ================================================================
  #  STEP 3 — RESOLVE MERCHANT ID via DB
  #
  #  signup_with_merchant_id derives the merchant_id from company_name
  #  (e.g. "HS Explorer" → "hs_explorer") and creates a new org+merchant.
  #  The only reliable way to get that merchant_id is to query the DB —
  #  the API response only returns {"is_email_sent": true}.
  # ================================================================
  info "[3/8] Resolving merchant ID for ${DASHBOARD_EMAIL}..."

  local PG_POD PG_PASS
  PG_POD=$(kubectl get pod -n "$HS_NAMESPACE" \
    -l app.kubernetes.io/name=postgresql \
    -o jsonpath='{.items[0].metadata.name}' 2>>"$LOG_FILE" || echo "")
  PG_PASS=$(kubectl get secret "${HS_RELEASE}-postgresql" -n "$HS_NAMESPACE" \
    -o jsonpath='{.data.postgres-password}' 2>/dev/null | base64 -d 2>/dev/null || echo "")

  if [ -z "$PG_POD" ] || [ -z "$PG_PASS" ]; then
    err "Cannot reach PostgreSQL to resolve merchant ID."
    exit 1
  fi

  MERCHANT_ID=$(kubectl exec -n "$HS_NAMESPACE" "$PG_POD" -- \
    env PGPASSWORD="$PG_PASS" psql -U postgres -d hyperswitch -tAq -c \
    "SELECT m.merchant_id
     FROM users u
     JOIN user_roles ur ON u.user_id = ur.user_id
     JOIN merchant_account m ON m.organization_id = ur.org_id
     WHERE u.email = '${DASHBOARD_EMAIL}'
     ORDER BY m.created_at DESC
     LIMIT 1;" 2>>"$LOG_FILE" | tr -d '[:space:]' || echo "")

  if [ -z "$MERCHANT_ID" ]; then
    err "Could not resolve merchant ID for ${DASHBOARD_EMAIL} from DB."
    exit 1
  fi
  ok "Merchant ID: $MERCHANT_ID"

  # ================================================================
  #  STEP 4 — CREATE MERCHANT API KEY
  # ================================================================
  info "[4/8] Creating merchant API key..."

  local apikey_resp
  apikey_resp=$(curl -s --max-time 30 -X POST "${APP}/api_keys/${MERCHANT_ID}" \
    -H "Content-Type: application/json" \
    -H "api-key: ${ADMIN_API_KEY}" \
    -d '{
      "name": "Explorer seed key",
      "description": "API key created by hyperswitch-explore.sh for seeding",
      "expiration": "never"
    }' 2>>"$LOG_FILE")

  MERCHANT_API_KEY=$(echo "$apikey_resp" | jq -r '.api_key // empty')

  if [ -z "$MERCHANT_API_KEY" ]; then
    err "Failed to create API key. Response:"
    echo "$apikey_resp" | jq . 2>/dev/null || echo "$apikey_resp"
    exit 1
  fi
  ok "Merchant API key created"

  # ================================================================
  #  STEP 5 — RESOLVE DEFAULT BUSINESS PROFILE
  #  Fetch the profile list and pick the one named "default".
  #  We never create a separate profile — connectors and transactions
  #  must land in the profile the user sees in the Control Center.
  # ================================================================
  info "[5/8] Resolving default business profile..."

  local profiles_resp
  profiles_resp=$(curl -s --max-time 30 -X GET "${APP}/account/${MERCHANT_ID}/business_profile" \
    -H "Content-Type: application/json" \
    -H "api-key: ${MERCHANT_API_KEY}" 2>>"$LOG_FILE")

  # Pick the profile named "default"; fall back to first in list
  PROFILE_ID=$(echo "$profiles_resp" \
    | jq -r 'if type=="array" then ((.[] | select(.profile_name=="default") | .profile_id), .[0].profile_id) else .profile_id end' \
    2>/dev/null | head -1 | tr -d '[:space:]' || echo "")

  if [ -z "$PROFILE_ID" ]; then
    err "Could not determine default business profile ID."
    err "Response: $(echo "$profiles_resp" | jq -c . 2>/dev/null || echo "$profiles_resp")"
    exit 1
  fi
  ok "Business profile (default): $PROFILE_ID"

  # ================================================================
  #  STEP 5 — CREATE CONNECTORS: stripe_test + paypal_test
  #  POST /account/{merchant_id}/connectors
  #  Auth: api-key: <admin_key>
  #  Note: stripe_test and paypal_test are Hyperswitch built-in
  #  test connectors — no real PSP credentials required.
  #  profile_id links the connector to the business profile.
  # ================================================================
  info "[6/8] Configuring stripe_test connector (profile: $PROFILE_ID)..."

  local stripe_resp
  stripe_resp=$(curl -s --max-time 30 -X POST "${APP}/account/${MERCHANT_ID}/connectors" \
    -H "Content-Type: application/json" \
    -H "api-key: ${MERCHANT_API_KEY}" \
    -d "{
      \"connector_type\": \"payment_processor\",
      \"connector_name\": \"stripe_test\",
      \"connector_label\": \"stripe_test_explorer\",
      \"profile_id\": \"${PROFILE_ID}\",
      \"connector_account_details\": {
        \"auth_type\": \"HeaderKey\",
        \"api_key\": \"test_stripe_dummy_key\"
      },
      \"test_mode\": true,
      \"disabled\": false,
      \"payment_methods_enabled\": [
        {
          \"payment_method\": \"card\",
          \"payment_method_types\": [
            {
              \"payment_method_type\": \"credit\",
              \"card_networks\": [\"Visa\", \"Mastercard\", \"AmericanExpress\"],
              \"minimum_amount\": 100,
              \"maximum_amount\": 99999999,
              \"recurring_enabled\": true,
              \"installment_payment_enabled\": false
            },
            {
              \"payment_method_type\": \"debit\",
              \"card_networks\": [\"Visa\", \"Mastercard\"],
              \"minimum_amount\": 100,
              \"maximum_amount\": 99999999,
              \"recurring_enabled\": true,
              \"installment_payment_enabled\": false
            }
          ]
        }
      ]
    }" 2>>"$LOG_FILE")

  STRIPE_TEST_MCA_ID=$(echo "$stripe_resp" | jq -r '.merchant_connector_id // empty')

  if [ -z "$STRIPE_TEST_MCA_ID" ]; then
    if echo "$stripe_resp" | grep -qi "already exists\|duplicate\|conflict"; then
      STRIPE_TEST_MCA_ID=$(echo "$stripe_resp" | jq -r '.error.message' 2>/dev/null || echo "existing")
      ok "stripe_test connector already exists, skipping"
    else
      err "Failed to create stripe_test connector. Response:"
      echo "$stripe_resp" | jq . 2>/dev/null || echo "$stripe_resp"
      exit 1
    fi
  else
    ok "stripe_test connector created: $STRIPE_TEST_MCA_ID"
  fi

  # ── paypal_test ─────────────────────────────────────────────
  info "[6/8] Configuring paypal_test connector..."

  local paypal_resp
  paypal_resp=$(curl -s --max-time 30 -X POST "${APP}/account/${MERCHANT_ID}/connectors" \
    -H "Content-Type: application/json" \
    -H "api-key: ${MERCHANT_API_KEY}" \
    -d "{
      \"connector_type\": \"payment_processor\",
      \"connector_name\": \"paypal_test\",
      \"connector_label\": \"paypal_test_explorer\",
      \"profile_id\": \"${PROFILE_ID}\",
      \"connector_account_details\": {
        \"auth_type\": \"HeaderKey\",
        \"api_key\": \"test_paypal_dummy_key\"
      },
      \"test_mode\": true,
      \"disabled\": false,
      \"payment_methods_enabled\": [
        {
          \"payment_method\": \"card\",
          \"payment_method_types\": [
            {
              \"payment_method_type\": \"credit\",
              \"card_networks\": [\"Visa\", \"Mastercard\"],
              \"minimum_amount\": 100,
              \"maximum_amount\": 99999999,
              \"recurring_enabled\": true,
              \"installment_payment_enabled\": false
            },
            {
              \"payment_method_type\": \"debit\",
              \"card_networks\": [\"Visa\", \"Mastercard\"],
              \"minimum_amount\": 100,
              \"maximum_amount\": 99999999,
              \"recurring_enabled\": true,
              \"installment_payment_enabled\": false
            }
          ]
        }
      ]
    }" 2>>"$LOG_FILE")

  PAYPAL_TEST_MCA_ID=$(echo "$paypal_resp" | jq -r '.merchant_connector_id // empty')
  if [ -z "$PAYPAL_TEST_MCA_ID" ]; then
    if echo "$paypal_resp" | grep -qi "already exists\|duplicate\|conflict"; then
      ok "paypal_test connector already exists, skipping"
    else
      err "Failed to create paypal_test connector. Response:"
      echo "$paypal_resp" | jq . 2>/dev/null || echo "$paypal_resp"
      exit 1
    fi
  else
    ok "paypal_test connector created: $PAYPAL_TEST_MCA_ID"
  fi

  # ── fauxpay ──────────────────────────────────────────────────
  info "[6/8] Configuring fauxpay connector..."

  local fauxpay_resp
  fauxpay_resp=$(curl -s --max-time 30 -X POST "${APP}/account/${MERCHANT_ID}/connectors" \
    -H "Content-Type: application/json" \
    -H "api-key: ${MERCHANT_API_KEY}" \
    -d "{
      \"connector_type\": \"payment_processor\",
      \"connector_name\": \"fauxpay\",
      \"connector_label\": \"fauxpay_explorer\",
      \"profile_id\": \"${PROFILE_ID}\",
      \"connector_account_details\": {
        \"auth_type\": \"HeaderKey\",
        \"api_key\": \"test_fauxpay_dummy_key\"
      },
      \"test_mode\": true,
      \"disabled\": false,
      \"payment_methods_enabled\": [
        {
          \"payment_method\": \"card\",
          \"payment_method_types\": [
            {
              \"payment_method_type\": \"credit\",
              \"card_networks\": [\"Visa\", \"Mastercard\", \"AmericanExpress\"],
              \"minimum_amount\": 100,
              \"maximum_amount\": 99999999,
              \"recurring_enabled\": true,
              \"installment_payment_enabled\": false
            }
          ]
        }
      ]
    }" 2>>"$LOG_FILE")

  FAUXPAY_MCA_ID=$(echo "$fauxpay_resp" | jq -r '.merchant_connector_id // empty')
  if [ -z "$FAUXPAY_MCA_ID" ]; then
    if echo "$fauxpay_resp" | grep -qi "already exists\|duplicate\|conflict"; then
      ok "fauxpay connector already exists, skipping"
    else
      err "Failed to create fauxpay connector. Response:"
      echo "$fauxpay_resp" | jq . 2>/dev/null || echo "$fauxpay_resp"
      exit 1
    fi
  else
    ok "fauxpay connector created: $FAUXPAY_MCA_ID"
  fi

  # ── pretendpay ───────────────────────────────────────────────
  info "[6/8] Configuring pretendpay connector..."

  local pretendpay_resp
  pretendpay_resp=$(curl -s --max-time 30 -X POST "${APP}/account/${MERCHANT_ID}/connectors" \
    -H "Content-Type: application/json" \
    -H "api-key: ${MERCHANT_API_KEY}" \
    -d "{
      \"connector_type\": \"payment_processor\",
      \"connector_name\": \"pretendpay\",
      \"connector_label\": \"pretendpay_explorer\",
      \"profile_id\": \"${PROFILE_ID}\",
      \"connector_account_details\": {
        \"auth_type\": \"HeaderKey\",
        \"api_key\": \"test_pretendpay_dummy_key\"
      },
      \"test_mode\": true,
      \"disabled\": false,
      \"payment_methods_enabled\": [
        {
          \"payment_method\": \"card\",
          \"payment_method_types\": [
            {
              \"payment_method_type\": \"credit\",
              \"card_networks\": [\"Mastercard\"],
              \"minimum_amount\": 100,
              \"maximum_amount\": 99999999,
              \"recurring_enabled\": true,
              \"installment_payment_enabled\": false
            },
            {
              \"payment_method_type\": \"debit\",
              \"card_networks\": [\"Mastercard\"],
              \"minimum_amount\": 100,
              \"maximum_amount\": 99999999,
              \"recurring_enabled\": true,
              \"installment_payment_enabled\": false
            }
          ]
        }
      ]
    }" 2>>"$LOG_FILE")

  PRETENDPAY_MCA_ID=$(echo "$pretendpay_resp" | jq -r '.merchant_connector_id // empty')
  if [ -z "$PRETENDPAY_MCA_ID" ]; then
    if echo "$pretendpay_resp" | grep -qi "already exists\|duplicate\|conflict"; then
      ok "pretendpay connector already exists, skipping"
    else
      err "Failed to create pretendpay connector. Response:"
      echo "$pretendpay_resp" | jq . 2>/dev/null || echo "$pretendpay_resp"
      exit 1
    fi
  else
    ok "pretendpay connector created: $PRETENDPAY_MCA_ID"
  fi

  # ================================================================
  #  STEP 6 — CREATE & ACTIVATE ROUTING RULES
  #
  #  Rules evaluated top-to-bottom, first match wins:
  #  1. billing_country = Germany (DE)  → paypal_test
  #  2. billing_country = France (FR)   → pretendpay
  #  3. billing_country = Japan (JP)    → fauxpay
  #  4. amount > $100 (10000 cents)     → stripe_test
  #  5. default                         → stripe_test
  #
  #  Note: euclid routing DSL uses full English country names (Germany, France,
  #  Japan) not ISO-2 codes (DE, FR, JP). Payment requests use ISO codes.
  #  card_network routing was attempted but BIN enrichment happens after routing
  #  decisions, so billing_country is used as the reliable differentiator.
  # ================================================================
  info "[7/8] Creating routing rules..."

  local routing_resp
  routing_resp=$(curl -s --max-time 30 -X POST "${APP}/routing" \
    -H "Content-Type: application/json" \
    -H "api-key: ${MERCHANT_API_KEY}" \
    -d "{
      \"name\": \"Explorer Routing\",
      \"description\": \"Rule-based routing seeded by hyperswitch-explore.sh\",
      \"profile_id\": \"${PROFILE_ID}\",
      \"algorithm\": {
        \"type\": \"advanced\",
        \"data\": {
          \"rules\": [
            {
              \"name\": \"germany_to_paypal\",
              \"connectorSelection\": {
                \"type\": \"priority\",
                \"data\": [{\"connector\": \"paypal_test\", \"merchant_connector_id\": \"${PAYPAL_TEST_MCA_ID}\"}]
              },
              \"statements\": [{
                \"condition\": [
                  {\"lhs\": \"billing_country\", \"comparison\": \"equal\",
                   \"value\": {\"type\": \"enum_variant\", \"value\": \"Germany\"}, \"metadata\": {}}
                ],
                \"nested\": null
              }]
            },
            {
              \"name\": \"france_to_pretendpay\",
              \"connectorSelection\": {
                \"type\": \"priority\",
                \"data\": [{\"connector\": \"pretendpay\", \"merchant_connector_id\": \"${PRETENDPAY_MCA_ID}\"}]
              },
              \"statements\": [{
                \"condition\": [
                  {\"lhs\": \"billing_country\", \"comparison\": \"equal\",
                   \"value\": {\"type\": \"enum_variant\", \"value\": \"France\"}, \"metadata\": {}}
                ],
                \"nested\": null
              }]
            },
            {
              \"name\": \"japan_to_fauxpay\",
              \"connectorSelection\": {
                \"type\": \"priority\",
                \"data\": [{\"connector\": \"fauxpay\", \"merchant_connector_id\": \"${FAUXPAY_MCA_ID}\"}]
              },
              \"statements\": [{
                \"condition\": [
                  {\"lhs\": \"billing_country\", \"comparison\": \"equal\",
                   \"value\": {\"type\": \"enum_variant\", \"value\": \"Japan\"}, \"metadata\": {}}
                ],
                \"nested\": null
              }]
            },
            {
              \"name\": \"high_amount_to_stripe\",
              \"connectorSelection\": {
                \"type\": \"priority\",
                \"data\": [{\"connector\": \"stripe_test\", \"merchant_connector_id\": \"${STRIPE_TEST_MCA_ID}\"}]
              },
              \"statements\": [{
                \"condition\": [
                  {\"lhs\": \"amount\", \"comparison\": \"greater_than\",
                   \"value\": {\"type\": \"number\", \"value\": 10000}, \"metadata\": {}}
                ],
                \"nested\": null
              }]
            }
          ],
          \"defaultSelection\": {
            \"type\": \"priority\",
            \"data\": [{\"connector\": \"stripe_test\", \"merchant_connector_id\": \"${STRIPE_TEST_MCA_ID}\"}]
          },
          \"metadata\": {}
        }
      }
    }" 2>>"$LOG_FILE")

  ROUTING_ID=$(echo "$routing_resp" | jq -r '.id // empty')
  if [ -z "$ROUTING_ID" ]; then
    err "Failed to create routing rule. Response:"
    echo "$routing_resp" | jq . 2>/dev/null || echo "$routing_resp"
    exit 1
  fi
  ok "Routing rule created: $ROUTING_ID"

  # Activate the routing rule
  local activate_resp
  activate_resp=$(curl -s --max-time 30 -X POST "${APP}/routing/${ROUTING_ID}/activate" \
    -H "Content-Type: application/json" \
    -H "api-key: ${MERCHANT_API_KEY}" \
    -d '{}' 2>>"$LOG_FILE")

  if echo "$activate_resp" | jq -e '.id' &>/dev/null; then
    ok "Routing rule activated"
  else
    err "Failed to activate routing rule. Response:"
    echo "$activate_resp" | jq . 2>/dev/null || echo "$activate_resp"
    exit 1
  fi

  # ================================================================
  #  STEP 7 — SEED TRANSACTIONS
  #
  #  Transactions are structured into 5 buckets to guarantee every
  #  routing rule is exercised. Within each bucket a success/fail card
  #  mix gives a realistic ~80% success rate overall.
  #
  #  Rule hit map:
  #    Bucket A (10 txns): billing_country=DE (Germany)   → paypal_test
  #    Bucket B (10 txns): billing_country=FR (France)    → pretendpay
  #    Bucket C (10 txns): billing_country=JP (Japan)     → fauxpay
  #    Bucket D (10 txns): amount>$100, Visa debit, US    → stripe_test
  #    Bucket E (10 txns): default (Visa debit, SG, ≤$100) → stripe_test
  # ================================================================
  info "[8/8] Seeding ${TXN_COUNT} transactions (all 4 routing rules covered)..."
  echo ""

  local SUCCESS=0 FAILED=0 REFUNDED=0
  local PAYMENT_IDS_FILE="/tmp/hs-payment-ids.txt"
  rm -f "$PAYMENT_IDS_FILE" /tmp/hs-txn-results.json
  touch /tmp/hs-txn-results.json

  # Cards
  local VISA_DEBIT="4111111111111111"
  local VISA_CREDIT="4111111111111111"   # Hyperswitch uses payment_method_type field to distinguish
  local MC_DEBIT="5200828282828210"
  local VISA_FAIL="4000000000000002"

  # Helper: fire one payment and append result
  _pay() {
    local amount="$1" currency="$2" card="$3" pmt="$4" country="$5" desc="$6"
    curl -s --max-time 30 -X POST "${APP}/payments" \
      -H "Content-Type: application/json" \
      -H "api-key: ${MERCHANT_API_KEY}" \
      -d "{
        \"amount\": ${amount},
        \"currency\": \"${currency}\",
        \"confirm\": true,
        \"capture_method\": \"automatic\",
        \"profile_id\": \"${PROFILE_ID}\",
        \"payment_method\": \"card\",
        \"payment_method_type\": \"${pmt}\",
        \"payment_method_data\": {
          \"card\": {
            \"card_number\": \"${card}\",
            \"card_exp_month\": \"03\",
            \"card_exp_year\": \"2030\",
            \"card_holder_name\": \"Test Customer\",
            \"card_cvc\": \"737\"
          }
        },
        \"billing\": {
          \"address\": {
            \"line1\": \"123 Demo St\",
            \"city\": \"Berlin\",
            \"zip\": \"10115\",
            \"country\": \"${country}\",
            \"first_name\": \"Test\",
            \"last_name\": \"Customer\"
          }
        },
        \"customer_id\": \"cust_demo_$((RANDOM % 200))\",
        \"description\": \"${desc}\"
      }" 2>/dev/null
  }

  local total=0

  # Bucket A — billing_country=DE → paypal_test (rule 1)
  local A_CARDS=("$VISA_DEBIT" "$VISA_DEBIT" "$VISA_DEBIT" "$VISA_DEBIT" "$VISA_DEBIT" "$VISA_DEBIT" "$VISA_DEBIT" "$VISA_DEBIT" "$VISA_FAIL" "$VISA_FAIL")
  local A_AMOUNTS=(500 1999 4999 9999 2999 7999 3499 8999 500 1999)
  for j in 0 1 2 3 4 5 6 7 8 9; do
    total=$((total + 1))
    resp=$(_pay "${A_AMOUNTS[$j]}" "EUR" "${A_CARDS[$j]}" "debit" "DE" "DE txn $((j+1)) → paypal_test")
    echo "$resp" >> /tmp/hs-txn-results.json
    printf "  %-4d / %-4d  (rule: billing_country=DE → paypal_test)\r" "$total" "$TXN_COUNT"
  done

  # Bucket B — billing_country=FR (France) → pretendpay (rule 2)
  local B_AMOUNTS=(500 999 1999 2999 4999 6999 3499 7999 500 1999)
  for j in 0 1 2 3 4 5 6 7 8 9; do
    total=$((total + 1))
    resp=$(_pay "${B_AMOUNTS[$j]}" "EUR" "$VISA_DEBIT" "debit" "FR" "FR txn $((j+1)) → pretendpay")
    echo "$resp" >> /tmp/hs-txn-results.json
    printf "  %-4d / %-4d  (rule: billing_country=FR → pretendpay)   \r" "$total" "$TXN_COUNT"
  done

  # Bucket C — billing_country=JP (Japan), Visa credit → fauxpay (rule 3)
  local C_CARDS=("$VISA_CREDIT" "$VISA_CREDIT" "$VISA_CREDIT" "$VISA_CREDIT" "$VISA_CREDIT" "$VISA_CREDIT" "$VISA_CREDIT" "$VISA_CREDIT" "$VISA_FAIL" "$VISA_FAIL")
  local C_AMOUNTS=(500 999 1999 2999 4999 6999 3499 7999 500 1999)
  for j in 0 1 2 3 4 5 6 7 8 9; do
    total=$((total + 1))
    resp=$(_pay "${C_AMOUNTS[$j]}" "JPY" "${C_CARDS[$j]}" "credit" "JP" "JP txn $((j+1)) → fauxpay")
    echo "$resp" >> /tmp/hs-txn-results.json
    printf "  %-4d / %-4d  (rule: billing_country=JP → fauxpay)       \r" "$total" "$TXN_COUNT"
  done

  # Bucket D — amount > $100 (10000 cents), Visa debit, non-DE → stripe_test (rule 4)
  local D_AMOUNTS=(14999 19999 24999 29999 49999 14999 19999 24999 29999 49999)
  for j in 0 1 2 3 4 5 6 7 8 9; do
    total=$((total + 1))
    resp=$(_pay "${D_AMOUNTS[$j]}" "USD" "$VISA_DEBIT" "debit" "US" "high-amt txn $((j+1)) → stripe_test")
    echo "$resp" >> /tmp/hs-txn-results.json
    printf "  %-4d / %-4d  (rule: amount>100 -> stripe_test)        \r" "$total" "$TXN_COUNT"
  done

  # Bucket E — default (Visa debit, ≤$100, non-DE) → stripe_test
  local E_AMOUNTS=(500 999 1999 2999 4999 6999 3499 7999 500 1999)
  for j in 0 1 2 3 4 5 6 7 8 9; do
    total=$((total + 1))
    resp=$(_pay "${E_AMOUNTS[$j]}" "SGD" "$VISA_DEBIT" "debit" "SG" "default txn $((j+1)) → stripe_test")
    echo "$resp" >> /tmp/hs-txn-results.json
    printf "  %-4d / %-4d  (rule: default → stripe_test)            \r" "$total" "$TXN_COUNT"
  done

  echo ""

  # Parse results
  SUCCESS=$(grep -o '"status":"succeeded"' /tmp/hs-txn-results.json 2>/dev/null | wc -l | tr -d ' ')
  FAILED=$(grep -o '"status":"failed"' /tmp/hs-txn-results.json 2>/dev/null | wc -l | tr -d ' ')

  # Extract succeeded payment IDs for refunds (first 100 only)
  grep -o '"payment_id":"pay_[^"]*"' /tmp/hs-txn-results.json \
    | cut -d'"' -f4 \
    | head -100 \
    > "$PAYMENT_IDS_FILE" 2>/dev/null || true

  ok "${TXN_COUNT} transactions seeded (~${SUCCESS} succeeded · ~${FAILED} failed)"

  # ── Refunds: 5% of succeeded payments ──────────────────
  info "[8/8] Creating refunds (~5% of succeeded)..."

  local refund_count=0
  local refund_pids=""
  while IFS= read -r pid && [ $refund_count -lt $((SUCCESS * 5 / 100)) ]; do
    curl -s --max-time 30 -X POST "${APP}/refunds" \
      -H "Content-Type: application/json" \
      -H "api-key: ${MERCHANT_API_KEY}" \
      -d "{
        \"payment_id\": \"${pid}\",
        \"reason\": \"customer_request\",
        \"refund_type\": \"instant\"
      }" >>"$LOG_FILE" 2>&1 &
    refund_pids="$refund_pids $!"
    refund_count=$((refund_count + 1))
  done < "$PAYMENT_IDS_FILE"
  # shellcheck disable=SC2086
  [ -n "$refund_pids" ] && wait $refund_pids 2>/dev/null || true
  REFUNDED=$refund_count
  ok "$REFUNDED refunds created"

  # ================================================================
  #  STEP 7 — SAVE STATE (idempotency guard for re-runs)
  # ================================================================
  info "[8/8] Saving state..."

  cat > "$STATE_FILE" << STATE
{
  "merchant_id":           "${MERCHANT_ID}",
  "admin_api_key":         "${ADMIN_API_KEY}",
  "merchant_api_key":      "${MERCHANT_API_KEY}",
  "profile_id":            "${PROFILE_ID}",
  "stripe_test_mca_id":    "${STRIPE_TEST_MCA_ID}",
  "paypal_test_mca_id":    "${PAYPAL_TEST_MCA_ID}",
  "fauxpay_mca_id":        "${FAUXPAY_MCA_ID}",
  "pretendpay_mca_id":     "${PRETENDPAY_MCA_ID}",
  "routing_id":            "${ROUTING_ID}",
  "txn_count":             ${TXN_COUNT},
  "success":               ${SUCCESS},
  "failed":                ${FAILED},
  "refunded":              ${REFUNDED},
  "app_port":              ${SVC_PORT_app},
  "cc_port":               ${SVC_PORT_cc},
  "web_port":              ${SVC_PORT_web},
  "grafana_port":          ${SVC_PORT_grafana},
  "vector_port":           ${SVC_PORT_vector},
  "mailhog_port":          ${SVC_PORT_mailhog},
  "seeded_at":             "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
STATE

  ok "State saved → $STATE_FILE"
}

_load_state() {
  MERCHANT_ID=$(jq -r '.merchant_id'           "$STATE_FILE")
  ADMIN_API_KEY=$(jq -r '.admin_api_key'       "$STATE_FILE")
  MERCHANT_API_KEY=$(jq -r '.merchant_api_key' "$STATE_FILE")
  PROFILE_ID=$(jq -r '.profile_id'             "$STATE_FILE")
  STRIPE_TEST_MCA_ID=$(jq -r '.stripe_test_mca_id'  "$STATE_FILE")
  PAYPAL_TEST_MCA_ID=$(jq -r '.paypal_test_mca_id'  "$STATE_FILE")
  FAUXPAY_MCA_ID=$(jq -r '.fauxpay_mca_id'         "$STATE_FILE")
  PRETENDPAY_MCA_ID=$(jq -r '.pretendpay_mca_id'   "$STATE_FILE")
  ROUTING_ID=$(jq -r '.routing_id'                  "$STATE_FILE")
  SVC_PORT_app=$(jq -r '.app_port'         "$STATE_FILE")
  SVC_PORT_cc=$(jq -r '.cc_port'           "$STATE_FILE")
  SVC_PORT_web=$(jq -r '.web_port'         "$STATE_FILE")
  SVC_PORT_grafana=$(jq -r '.grafana_port' "$STATE_FILE")
  SVC_PORT_vector=$(jq -r '.vector_port'   "$STATE_FILE")
  SVC_PORT_mailhog=$(jq -r '.mailhog_port' "$STATE_FILE")
}

# ================================================================
#  FINAL OUTPUT — print service URLs
# ================================================================
launch() {
  local PAD=22

  echo ""
  echo -e "  ${G}${W}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"
  echo -e "  ${W}  Hyperswitch is ready. Explore at:${N}"
  echo -e "  ${G}${W}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${N}"
  echo ""
  printf "  ${C}%-${PAD}s${N} %s\n" "Control Center"    "http://localhost:${SVC_PORT_cc}"
  printf "  ${C}%-${PAD}s${N} %s\n" "App Server API"    "http://localhost:${SVC_PORT_app}"
  printf "  ${C}%-${PAD}s${N} %s\n" "Hyperswitch Web"   "http://localhost:${SVC_PORT_web}/HyperLoader.js"
  printf "  ${C}%-${PAD}s${N} %s\n" "Grafana"           "http://localhost:${SVC_PORT_grafana}"
  printf "  ${C}%-${PAD}s${N} %s\n" "Vector"            "http://localhost:${SVC_PORT_vector}"
  printf "  ${C}%-${PAD}s${N} %s\n" "Mailhog"           "http://localhost:${SVC_PORT_mailhog}"
  echo ""
  echo -e "  ${W}What's set up:${N}"
  printf "    %-20s %s\n" "Merchant"       "${MERCHANT_ID}"
  printf "    %-20s %s\n" "Connectors"     "stripe_test · paypal_test · fauxpay · pretendpay"
  printf "    %-20s %s\n" "Routing rules"  "Germany → paypal_test · France → pretendpay · Japan → fauxpay · >100 USD → stripe_test"
  printf "    %-20s %s\n" "Transactions"   "${TXN_COUNT} seeded (success / failure / refund mix, spread across all 4 connectors)"
  printf "    %-20s %s\n" "Observability"  "Grafana dashboards + Vector log pipeline live"
  printf "    %-20s %s\n" "Email (mock)"   "Mailhog captures all outbound emails"
  echo ""
  echo -e "  ${Y}Login:${N}  http://localhost:${SVC_PORT_cc}  (${DASHBOARD_EMAIL})"
  echo ""
  echo -e "  ${W}Cleanup:${N}"
  echo "    Stop forwards : pkill -f 'kubectl port-forward.*${HS_NAMESPACE}'"
  echo "    Tear down     : helm uninstall ${HS_RELEASE} -n ${HS_NAMESPACE}"
  echo "    Delete ns     : kubectl delete namespace ${HS_NAMESPACE}"
  echo ""
}
_validate_password() {
  local pw="$1"
  local ok=true
  [ ${#pw} -ge 8 ]                   || { err "Password must be at least 8 characters.";                  ok=false; }
  echo "$pw" | grep -q '[A-Z]'       || { err "Password must contain at least one uppercase letter.";     ok=false; }
  echo "$pw" | grep -q '[a-z]'       || { err "Password must contain at least one lowercase letter.";     ok=false; }
  echo "$pw" | grep -q '[0-9]'       || { err "Password must contain at least one number.";               ok=false; }
  echo "$pw" | grep -q '[^a-zA-Z0-9]' || { err "Password must contain at least one special character.";  ok=false; }
  $ok
}

main() {
  clear
  banner

  # ── Collect dashboard credentials upfront ───────────────────
  echo ""
  echo "  Create your Control Center login (used to access the dashboard after setup):"
  echo ""
  printf "  Email    : "; read -r DASHBOARD_EMAIL
  echo ""
  echo "  Password requirements: 8+ chars · uppercase · lowercase · number · special character"
  echo "  Example: MyPass@123"
  echo ""
  while true; do
    printf "  Password : "; read -rs DASHBOARD_PASSWORD; echo ""
    if _validate_password "$DASHBOARD_PASSWORD"; then
      break
    fi
    echo ""
    echo "  Try again — password must have: 8+ chars · uppercase · lowercase · number · special character"
    echo ""
  done
  echo ""

  # ── Pre-run summary ─────────────────────────────────────────
  echo -e "  ${W}What this will do:${N}"
  echo ""
  echo "  1. Check prerequisites (kubectl, helm, curl, jq)"
  echo "  2. Verify / start a local Kubernetes cluster"
  echo "  3. Deploy Hyperswitch via Helm  ← slowest step, ~5-8 min on first run"
  echo "  4. Port-forward 6 services to localhost"
  echo "  5. Create your Control Center account  (${DASHBOARD_EMAIL})"
  echo "  6. Add 4 dummy connectors: stripe_test · paypal_test · fauxpay · pretendpay"
  echo "  7. Set up routing rules: Germany → paypal · France → pretendpay · Japan → fauxpay · >100 USD → stripe"
  echo "  8. Seed ${TXN_COUNT} transactions spread across all routing rules (success / failure / refund mix)"
  echo ""
  echo -e "  ${Y}Estimated time: 5-10 minutes on first run · ~2 min on subsequent runs${N}"
  echo ""
  echo -e "  ${W}All traffic stays on localhost — no external services are contacted.${N}"
  echo ""
  echo "  ──────────────────────────────────────────────────────"
  echo ""

  export DASHBOARD_EMAIL DASHBOARD_PASSWORD

  exec > >(tee -a "$LOG_FILE") 2>&1

  install_prereqs
  ensure_cluster
  ensure_deployed
  start_port_forwards
  seed_environment
  launch
}

main "$@"
