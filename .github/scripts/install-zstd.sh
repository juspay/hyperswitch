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

# hyperswitch-runners has no route to apt mirrors or PPAs (only github.com
# and crates.io are reachable), and upstream zstd doesn't publish a
# prebuilt Linux binary — so build the CLI from source instead. Only needs
# `make` and a C compiler, both already required to build this project.
zstd_version="v1.5.7"

work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT

git clone --quiet --depth 1 --branch "$zstd_version" \
  https://github.com/facebook/zstd.git "$work_dir/zstd"

make -C "$work_dir/zstd/programs" -j"$(nproc)" HAVE_LZMA=0 HAVE_LZ4=0 HAVE_ZLIB=0 zstd

mkdir -p ~/.local/bin
cp "$work_dir/zstd/programs/zstd" ~/.local/bin/zstd
chmod +x ~/.local/bin/zstd

echo "$HOME/.local/bin" >> "${GITHUB_PATH}"

~/.local/bin/zstd --version
