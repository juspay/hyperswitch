#! /usr/bin/env bash

# Shared timing/progress helpers for the sccache S3 transfer scripts.
#
# These exist because both scripts used to print one line and nothing else, so
# every question about their performance had to be answered by diffing
# GitHub's step durations. That repeatedly produced wrong answers — most
# recently a 4.5 s "save" of a 2 GiB cache that step timings alone could not
# explain (compressed to something small? uploaded at 450 MB/s? silently
# truncated?). Log bytes and seconds per stage instead of inferring them.
#
# Sourced, not executed. GNU coreutils assumed (Linux CI runners only).

# Milliseconds since the epoch.
now_ms() { date +%s%3N; }

# bytes_of <path> -- size of a file, or apparent total of a directory tree.
bytes_of() {
  if [ -d "$1" ]; then
    du -sb "$1" 2>/dev/null | cut -f1
  else
    stat -c %s "$1" 2>/dev/null || echo 0
  fi
}

# stage_report <label> <bytes> <elapsed_ms>
stage_report() {
  awk -v l="$1" -v b="$2" -v ms="$3" 'BEGIN {
    if (ms <= 0) ms = 1
    printf "  %-22s %9.1f MiB  in %7.1fs  =  %7.1f MiB/s\n", \
      l, b / 1048576, ms / 1000, (b / 1048576) / (ms / 1000)
  }'
}

# ratio_report <label> <uncompressed_bytes> <compressed_bytes>
ratio_report() {
  awk -v l="$1" -v u="$2" -v c="$3" 'BEGIN {
    if (c <= 0) c = 1
    printf "  %-22s %9.1f MiB -> %.1f MiB  (%.2fx)\n", \
      l, u / 1048576, c / 1048576, u / c
  }'
}

# s3_progress [stride]
#
# Filter for `aws s3 cp` output, which is suppressed entirely by the
# --only-show-errors these scripts used to pass. Drop that flag and pipe
# through this instead.
#
# The CLI updates progress in place with carriage returns, which a non-TTY
# Actions log renders as one enormous unreadable line — so translate \r to
# newlines and print every <stride>th update, keeping anything that looks
# like an error. A fast transfer may produce no progress lines at all, which
# is fine: the stage totals still get logged either way.
#
# Note this reports bytes actually transferred. Sampling the local file's
# size would be wrong for downloads: parallel ranged GETs write out of order,
# so the file reaches full size well before the transfer completes.
s3_progress() {
  local stride="${1:-20}"
  tr '\r' '\n' | awk -v n="$stride" 'NR % n == 0 || tolower($0) ~ /error|warn/'
}
