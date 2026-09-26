#! /usr/bin/env bash

set -euo pipefail

if [[ "${CI:-false}" != "true" && "${GITHUB_ACTIONS:-false}" != "true" ]]; then
  echo "This script is to be run in a GitHub Actions runner only. Exiting."
  exit 1
fi

if command -v zstd &>/dev/null; then
  echo "zstd already present: $(zstd --version)"
  exit 0
fi

apt_get() {
  if [ "$(id -u)" -eq 0 ]; then
    apt-get "$@"
  else
    sudo apt-get "$@"
  fi
}

apt_get update -qq
apt_get install -y -qq zstd
