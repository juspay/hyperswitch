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

# ---------------------------------------------------------------------------
# TCP path proof
#
# Direct evidence of whether S3 bytes traverse the egress proxy, by observing
# the pod's actual sockets during a real transfer. This is stronger than
# asking botocore what it intends: it looks at the connections themselves.
#
# The discriminator is the socket the pod opens, and it is unambiguous:
#
#   through an HTTP proxy -> TCP to <proxy>:80, carrying a CONNECT tunnel.
#                            The pod never opens :443 itself.
#   direct                -> TCP to an S3 address on :443.
#
# Classifying on port rather than on address also makes this immune to S3's
# DNS rotation, which hands back a different IP set on every lookup.

# peer_sampler_start <outfile> -- echoes the sampler's pid.
peer_sampler_start() {
  local out="$1"
  : >"$out"
  if ! command -v ss >/dev/null 2>&1 && ! command -v python3 >/dev/null 2>&1; then
    echo ""
    return 0
  fi
  (
    while :; do
      if command -v ss >/dev/null 2>&1; then
        ss -tn 2>/dev/null | awk 'NR > 1 { print $NF }'
      else
        python3 - <<'PY'
import socket, struct
for proto, path in (("4", "/proc/net/tcp"), ("6", "/proc/net/tcp6")):
    try:
        rows = open(path).read().splitlines()[1:]
    except OSError:
        continue
    for row in rows:
        f = row.split()
        if len(f) < 4 or f[3] != "01":  # 01 = ESTABLISHED
            continue
        host, _, port = f[2].partition(":")
        if proto == "4":
            ip = socket.inet_ntoa(struct.pack("<L", int(host, 16)))
        else:
            b = bytes.fromhex(host)
            ip = socket.inet_ntop(
                socket.AF_INET6,
                b"".join(b[i:i + 4][::-1] for i in range(0, 16, 4)),
            )
        print(f"{ip}:{int(port, 16)}")
PY
      fi
      sleep 0.3
    done
  ) >>"$out" 2>/dev/null &
  echo $!
}

# peer_verdict <sampler_pid> <outfile> <proxy_url>
peer_verdict() {
  local pid="$1" out="$2" proxy_url="$3"
  local proxy_host proxy_ips proxy_n tls_n peers

  if [ -n "$pid" ]; then
    kill "$pid" 2>/dev/null || true
    wait "$pid" 2>/dev/null || true
  fi

  echo "  --- TCP path proof (sampled during the transfer above) ---"

  if [ ! -s "$out" ]; then
    echo "      no socket samples captured (no ss/python3?) — inconclusive"
    return 0
  fi

  proxy_host="${proxy_url#*://}"
  proxy_host="${proxy_host%%:*}"
  proxy_ips="$(getent ahostsv4 "$proxy_host" 2>/dev/null | awk '{print $1}' | sort -u | paste -sd'|' -)"
  [ -z "$proxy_ips" ] && proxy_ips="__none__"

  # S3 is always a public address, so :443 sockets to RFC1918/link-local peers
  # are unrelated in-cluster chatter (runner agent, kubelet, sidecars) and must
  # not be counted as evidence of a direct path — otherwise the proxied case
  # reports MIXED precisely when a clear verdict is needed.
  local private='^(10\.|127\.|192\.168\.|169\.254\.|172\.(1[6-9]|2[0-9]|3[01])\.|\[|::)'

  # Normalise IPv4-mapped IPv6 (`::ffff:10.1.91.193:80`) to plain IPv4 first.
  # /proc/net/tcp6 reports dual-stack sockets in that form, and a run can show
  # a peer *only* that way — which otherwise escapes both the proxy match and
  # the private-range filter, under-counting proxy connections.
  peers="$(sed -e 's/^\[::ffff:\([0-9.]*\)\]/\1/' -e 's/^::ffff://' "$out" |
    sort -u | grep -v '^$' || true)"
  proxy_n="$(printf '%s\n' "$peers" | grep -cE "^(${proxy_ips}):" || true)"
  tls_n="$(printf '%s\n' "$peers" | grep -E ':443$' | grep -cvE "$private" || true)"

  echo "      proxy ${proxy_host} -> ${proxy_ips//|/ }"
  echo "      distinct sockets to the proxy         : ${proxy_n}"
  echo "      distinct public TLS sockets (S3, :443): ${tls_n}"
  echo "      peers observed (· = private, ignored):"
  printf '%s\n' "$peers" | grep -E "$private" | sed 's/^/        · /'
  printf '%s\n' "$peers" | grep -vE "$private" | sed 's/^/          /'

  # Public TLS sockets decide it. Opening :443 straight to S3 during an S3
  # transfer means the bytes went direct — botocore makes one proxy decision
  # per host, so it cannot also be tunnelling them.
  #
  # A proxy socket alongside them is expected, not contradictory: credentials
  # still resolve through the proxy, because a NO_PROXY entry of
  # `.s3.<region>.amazonaws.com` does not match `sts.<region>.amazonaws.com`.
  # Treating that coexistence as "mixed" would report inconclusive on a run
  # that is in fact cleanly direct.
  if [ "$tls_n" -gt 0 ]; then
    echo "      VERDICT: DIRECT — S3 bytes bypass the proxy"
    [ "$proxy_n" -gt 0 ] &&
      echo "               (${proxy_n} proxy socket(s) remain — credentials/STS, expected)"
  elif [ "$proxy_n" -gt 0 ]; then
    echo "      VERDICT: PROXIED — S3 bytes traverse ${proxy_host}"
  else
    echo "      VERDICT: INCONCLUSIVE — no S3 or proxy peers seen; see above"
  fi
}
