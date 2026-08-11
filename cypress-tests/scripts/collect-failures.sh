#!/usr/bin/env bash
#
# collect-failures.sh — aggregate Cypress failures across every connector.
#
# Reads the mochawesome JSON produced by cypress.config.js
# (cypress/reports/<connector>/<connector>_report*.json) and emits:
#
#   <out-dir>/connector-failures.json   machine readable, for triage/CI
#   <out-dir>/connector-failures.html   self contained report, no external assets
#
# Meant to run once at the end of a pipeline, after every connector matrix job
# has dropped its reports into cypress/reports/.
#
# The default out-dir is the reports root itself, so both CIs pick the report up
# with no pipeline change: they already archive cypress-tests/cypress/reports/.
# Writing two loose files there is safe — this script and report-generator.js
# both enumerate directories only, so neither mistakes them for a connector.
#
# Usage: ./scripts/collect-failures.sh [options]
#   -r, --reports-dir DIR   mochawesome root            (default: cypress/reports)
#   -o, --out-dir DIR       output directory            (default: the reports root)
#   -c, --connector NAME    only this connector, repeatable
#   -s, --since TIMESTAMP   ignore runs that ended before this ISO-8601 instant
#       --latest            keep only the newest run per connector
#       --no-dedupe         aggregate every run instead of newest-per-spec
#       --stack-lines N     stack trace lines to keep      (default: 12)
#       --max-message N     error message chars to keep    (default: 1200)
#       --fail-on-failure   exit 1 when any failure was collected
#   -q, --quiet             suppress the per-file progress log
#   -h, --help              show this help
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

REPORTS_DIR="${CYPRESS_REPORTS_DIR:-${ROOT_DIR}/cypress/reports}"
OUT_DIR="${FAILURE_REPORT_DIR:-}"   # empty = default into the reports root, resolved below
SINCE=""
DEDUPE="true"
LATEST_ONLY="false"
STACK_LINES=12
MAX_MESSAGE=1200
FAIL_ON_FAILURE="false"
QUIET="false"
CONNECTOR_FILTER=()

die() {
  printf '❌ %s\n' "$*" >&2
  exit 1
}
log() {
  [ "$QUIET" = "true" ] || printf '%s\n' "$*" >&2
}
usage() {
  sed -n '2,30p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
  exit 0
}

while [ $# -gt 0 ]; do
  case "$1" in
    -r | --reports-dir) REPORTS_DIR="${2:?--reports-dir needs a value}"; shift 2 ;;
    -o | --out-dir)     OUT_DIR="${2:?--out-dir needs a value}"; shift 2 ;;
    -c | --connector)   CONNECTOR_FILTER+=("${2:?--connector needs a value}"); shift 2 ;;
    -s | --since)       SINCE="${2:?--since needs a value}"; shift 2 ;;
    --latest)           LATEST_ONLY="true"; shift ;;
    --no-dedupe)        DEDUPE="false"; shift ;;
    --stack-lines)      STACK_LINES="${2:?--stack-lines needs a value}"; shift 2 ;;
    --max-message)      MAX_MESSAGE="${2:?--max-message needs a value}"; shift 2 ;;
    --fail-on-failure)  FAIL_ON_FAILURE="true"; shift ;;
    -q | --quiet)       QUIET="true"; shift ;;
    -h | --help)        usage ;;
    *) die "Unknown argument: $1 (try --help)" ;;
  esac
done

command -v jq >/dev/null 2>&1 || die "jq is required but not installed (brew install jq / apt-get install jq)"
[ -d "$REPORTS_DIR" ] || die "Reports directory not found: $REPORTS_DIR"

# Fail loudly on an unparseable --since; silently keeping every run would make a
# CI report look like it covered the pipeline when it actually covered history.
if [ -n "$SINCE" ]; then
  jq -ne --arg s "$SINCE" '$s | sub("\\.[0-9]+"; "") | fromdateiso8601' >/dev/null 2>&1 ||
    die "--since must be UTC ISO-8601 like 2026-08-10T09:00:00Z (got: ${SINCE})"
fi

REPORTS_DIR="$(cd "$REPORTS_DIR" && pwd)"

# Resolved here rather than at declaration so that an explicit --reports-dir still
# lands the report next to the reports it summarises.
OUT_DIR="${OUT_DIR:-$REPORTS_DIR}"
mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"

JSON_OUT="${OUT_DIR}/connector-failures.json"
HTML_OUT="${OUT_DIR}/connector-failures.html"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cypress-failures.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT
RUNS_NDJSON="${TMP_DIR}/runs.ndjson"
: >"$RUNS_NDJSON"

# Build into the temp dir and rename into place at the end. Several connectors can
# finish at once (Cypress after:run, CI batches running in parallel), and a plain
# truncating write would let two collectors interleave into the same file.
JSON_TMP="${TMP_DIR}/connector-failures.json"
HTML_TMP="${TMP_DIR}/connector-failures.html"

# ---------------------------------------------------------------------------
# Stage 1 — project each mochawesome file down to a compact run record.
#
# Done one file at a time on purpose: embeddedScreenshots inlines base64 PNGs,
# so single reports reach ~100MB. This drops `context`/`code` before anything
# is buffered, keeping peak memory to one file.
# ---------------------------------------------------------------------------
read -r -d '' JQ_EXTRACT <<'JQ' || true
def strip_ansi: gsub("\\u001b\\[[0-9;]*m"; "");
def clip($n): if ($n > 0 and (length > $n)) then (.[0:$n] + " …[truncated]") else . end;
def as_text: (. // "") | tostring | strip_ansi;

# Screenshots arrive as a JSON-encoded string in `context`; count them, never
# carry the base64 payload forward.
def shot_count:
  (.context
   | if type == "string" then (fromjson? // null) else . end
   | if . == null then 0 elif type == "array" then length else 1 end);

def signature:
  split("\n")[0]
  | gsub("[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}"; "<uuid>")
  | gsub("[0-9]+"; "<n>")
  | gsub("\\s+"; " ")
  | clip(200);

# Not a mochawesome report (stray json in the folder) -> contribute nothing.
if (has("stats") and has("results")) | not then empty else
{
  source: $source,
  connector: $connector,
  runStart: (.stats.start // null),
  runEnd: (.stats.end // .stats.start // null),
  specs: [
    .results[]?
    | . as $res
    | {
        spec: ($res.file // $res.fullFile // "unknown"),
        durationMs: ($res.duration // 0),
        tests: [
          $res
          | recurse(.suites[]?) as $s
          | ( ($s.tests[]?        | . + {__hook: false}),
              ($s.beforeHooks[]?  | . + {__hook: true}),
              ($s.afterHooks[]?   | . + {__hook: true}) )
          | select(.title != null)
          # passing hooks are noise; a failing hook kills a whole suite, keep it
          | select((.__hook | not) or (.fail == true))
          | {
              suite: ($s.title // ""),
              title: .title,
              fullTitle: (.fullTitle // .title),
              state: (if .fail then "failed"
                      elif .pass then "passed"
                      elif .pending then "pending"
                      elif .skipped then "skipped"
                      else (.state // "unknown") end),
              hook: .__hook,
              durationMs: (.duration // 0),
              timedOut: (.timedOut // false),
              retries: (.currentRetry // 0),
              screenshots: shot_count,
              error: (if .fail then
                        (.err.message | as_text) as $msg
                        | {
                            message: ($msg | clip($maxMessage)),
                            signature: ($msg | signature),
                            stack: ((.err.estack // .err.stack) | as_text
                                    | split("\n") | .[0:$stackLines] | join("\n")),
                            diff: (if (.err.diff // null) == null then null
                                   else (.err.diff | as_text | clip($maxMessage)) end)
                          }
                      else null end)
            }
        ]
      }
  ]
}
end
JQ

shopt -s nullglob
connector_dirs=()
for dir in "$REPORTS_DIR"/*/; do
  name="$(basename "$dir")"
  # never treat the output folder as a connector if it lives under reports/
  [ "$dir" = "${OUT_DIR}/" ] && continue
  if [ ${#CONNECTOR_FILTER[@]} -gt 0 ]; then
    match="false"
    for wanted in ${CONNECTOR_FILTER[@]+"${CONNECTOR_FILTER[@]}"}; do
      [ "$name" = "$wanted" ] && match="true"
    done
    [ "$match" = "true" ] || continue
  fi
  connector_dirs+=("$dir")
done

[ ${#connector_dirs[@]} -gt 0 ] || log "⚠️  No connector directories found under ${REPORTS_DIR}"

total_files=0
parsed_files=0
skipped_files=0

# ${arr[@]+…} keeps `set -u` happy when nothing matched (bash 3.2 on macOS)
for dir in ${connector_dirs[@]+"${connector_dirs[@]}"}; do
  connector="$(basename "$dir")"
  report_files=("$dir"*.json)

  # cypress-mochawesome-reporter stages per-spec json in <connector>/.jsons/ and
  # merges it up on exit. If the merge never ran (interrupted job) that folder is
  # the only record of the run — fall back to it rather than lose the connector.
  if [ ${#report_files[@]} -eq 0 ]; then
    report_files=("$dir.jsons/"*.json)
    [ ${#report_files[@]} -gt 0 ] &&
      log "   ℹ️  ${connector}: no merged report, falling back to .jsons/"
  fi
  [ ${#report_files[@]} -gt 0 ] || continue

  log "📂 ${connector} (${#report_files[@]} report file(s))"
  for file in "${report_files[@]}"; do
    total_files=$((total_files + 1))
    if jq -c \
      --arg connector "$connector" \
      --arg source "${file#"$REPORTS_DIR"/}" \
      --argjson stackLines "$STACK_LINES" \
      --argjson maxMessage "$MAX_MESSAGE" \
      "$JQ_EXTRACT" "$file" >>"$RUNS_NDJSON" 2>"${TMP_DIR}/jq.err"; then
      parsed_files=$((parsed_files + 1))
    else
      skipped_files=$((skipped_files + 1))
      log "   ⚠️  skipped unreadable report: ${file#"$REPORTS_DIR"/} ($(tr '\n' ' ' <"${TMP_DIR}/jq.err"))"
    fi
  done
done

# ---------------------------------------------------------------------------
# Stage 2 — fold the run records into one failure summary.
# ---------------------------------------------------------------------------
read -r -d '' JQ_AGGREGATE <<'JQ' || true
def num($n): $n // 0;
def pct($part; $total): if $total == 0 then 0 else (($part / $total) * 1000 | round) / 10 end;

# Mochawesome timestamps carry milliseconds ("…:29.560Z") while a caller-supplied
# --since usually does not ("…:29Z"). Comparing those as strings is wrong — "." is
# below "Z" in ASCII, so a run finishing inside the cutoff second looks older than
# the cutoff. Compare epoch seconds instead. An unparseable run timestamp is kept
# rather than silently dropped.
def to_epoch: (. // "") | sub("\\.[0-9]+"; "") | (try fromdateiso8601 catch null);

($since | to_epoch) as $sinceEpoch

# runs -> flat list of (connector, spec) records
| [ .[]
  | select($since == "" or (((.runEnd | to_epoch) // 99999999999) >= $sinceEpoch))
  | . as $run
  | $run.specs[]
  | . + {connector: $run.connector, runEnd: ($run.runEnd // ""), source: $run.source}
] as $all

# --latest: throw away every run except the connector's most recent one
| (if $latestOnly then
     ($all | group_by(.connector)
           | map(. as $g | ($g | map(.runEnd) | max) as $newest
                 | $g | map(select(.runEnd == $newest)))
           | add // [])
   else $all end)

# default: one entry per (connector, spec), taken from the newest run that ran it
| (if $dedupe then
     group_by(.connector + "\u0000" + .spec) | map(max_by(.runEnd))
   else . end) as $specs

| ($specs
   | group_by(.connector)
   | map(
       (map(.tests[])) as $tests
       | ($tests | map(select(.state == "failed"))) as $failed
       | {
           connector: .[0].connector,
           specCount: length,
           lastRunEnd: (map(.runEnd) | max),
           stats: {
             tests: ($tests | length),
             passed: ($tests | map(select(.state == "passed")) | length),
             failed: ($failed | length),
             pending: ($tests | map(select(.state == "pending")) | length),
             skipped: ($tests | map(select(.state == "skipped")) | length),
             # results[].duration is always 0 in mochawesome; sum the tests instead
             durationMs: ($tests | map(.durationMs) | add // 0)
           },
           failures: [
             .[] as $spec
             | $spec.tests[]
             | select(.state == "failed")
             | {
                 spec: $spec.spec,
                 suite: .suite,
                 title: .title,
                 fullTitle: .fullTitle,
                 hook: .hook,
                 retries: .retries,
                 timedOut: .timedOut,
                 durationMs: .durationMs,
                 screenshots: .screenshots,
                 error: .error,
                 source: $spec.source
               }
           ]
         }
       # pass rate is over executed tests only — pending/skipped are not signal
       | .stats += {
           executed: (.stats.passed + .stats.failed),
           passPercent: pct(.stats.passed; .stats.passed + .stats.failed)
         }
     )
   | sort_by([(.stats.failed * -1), .connector])
  ) as $connectors

| ([ $connectors[] | .connector as $c | .failures[] | . + {connector: $c} ]) as $allFailures

| {
    generatedAt: $generatedAt,
    runId: $runId,
    reportsDir: $reportsDir,
    options: {
      dedupeBySpec: $dedupe,
      latestRunOnly: $latestOnly,
      since: (if $since == "" then null else $since end)
    },
    files: {scanned: $scanned, parsed: $parsed, skipped: $skippedFiles},
    totals: {
      connectors: ($connectors | length),
      connectorsWithFailures: ($connectors | map(select(.stats.failed > 0)) | length),
      specs: ($specs | length),
      tests: ($connectors | map(.stats.tests) | add // 0),
      passed: ($connectors | map(.stats.passed) | add // 0),
      failed: ($connectors | map(.stats.failed) | add // 0),
      pending: ($connectors | map(.stats.pending) | add // 0),
      skipped: ($connectors | map(.stats.skipped) | add // 0),
      durationMs: ($connectors | map(.stats.durationMs) | add // 0)
    }
    | . + {executed: (.passed + .failed), passPercent: pct(.passed; .passed + .failed)},
    failingConnectors: ($connectors | map(select(.stats.failed > 0) | .connector)),
    topErrors: (
      $allFailures
      | map(select(.error != null))
      | group_by(.error.signature)
      | map({
          signature: .[0].error.signature,
          count: length,
          connectors: (map(.connector) | unique),
          specs: (map(.spec) | unique),
          sample: {
            connector: .[0].connector,
            fullTitle: .[0].fullTitle,
            message: .[0].error.message
          }
        })
      | sort_by(-.count)
      | .[0:25]
    ),
    connectors: $connectors
  }
JQ

log "🧮 Aggregating ${parsed_files} report file(s)…"
jq -s \
  --arg generatedAt "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg runId "${GITHUB_RUN_ID:-${CI_PIPELINE_ID:-${BUILD_NUMBER:-local}}}" \
  --arg reportsDir "$REPORTS_DIR" \
  --arg since "$SINCE" \
  --argjson dedupe "$DEDUPE" \
  --argjson latestOnly "$LATEST_ONLY" \
  --argjson scanned "$total_files" \
  --argjson parsed "$parsed_files" \
  --argjson skippedFiles "$skipped_files" \
  "$JQ_AGGREGATE" "$RUNS_NDJSON" >"$JSON_TMP"

# ---------------------------------------------------------------------------
# Stage 3 — render the HTML from the JSON we just wrote.
# Static markup (readable with JS disabled); the filter box is progressive.
# ---------------------------------------------------------------------------
read -r -d '' JQ_HTML <<'JQ' || true
def h: (. // "") | tostring | @html;
def dur: (. // 0) | if . < 1000 then "\(.)ms"
                    elif . < 60000 then "\((. / 100 | round) / 10)s"
                    else "\((. / 6000 | round) / 10)m" end;
def slug: (. // "") | tostring | gsub("[^a-zA-Z0-9_-]"; "-");

. as $d
| "<header class=\"page-head\">",
  "  <h1>Cypress connector failures</h1>",
  "  <p class=\"sub\">run <code>\($d.runId | h)</code> · generated \($d.generatedAt | h) · \($d.files.parsed) report file(s) from <code>\($d.reportsDir | h)</code></p>",
  "</header>",
  "<section class=\"cards\">",
  "  <div class=\"card \(if $d.totals.failed > 0 then "bad" else "good" end)\"><span class=\"n\">\($d.totals.failed)</span><span class=\"l\">failed tests</span></div>",
  "  <div class=\"card\"><span class=\"n\">\($d.totals.connectorsWithFailures)/\($d.totals.connectors)</span><span class=\"l\">connectors failing</span></div>",
  "  <div class=\"card\"><span class=\"n\">\($d.totals.executed)</span><span class=\"l\">tests executed</span></div>",
  "  <div class=\"card\"><span class=\"n\">\($d.totals.passPercent)%</span><span class=\"l\">pass rate</span></div>",
  "  <div class=\"card\"><span class=\"n\">\($d.totals.durationMs | dur)</span><span class=\"l\">duration</span></div>",
  "</section>",

  ( if ($d.totals.failed == 0) then
      "<p class=\"empty\">✅ No failures collected. Every connector report is green.</p>"
    else empty end ),

  ( if ($d.topErrors | length) > 0 then
      ( "<section>",
        "<h2>Top failure signatures</h2>",
        "<table class=\"grid\">",
        "<thead><tr><th>#</th><th>Signature</th><th>Connectors</th></tr></thead><tbody>",
        ( $d.topErrors[]
          | "<tr><td class=\"num\">\(.count)</td><td><code>\(.signature | h)</code></td><td class=\"conns\">\(.connectors | map("<span class=\"chip\">" + (. | @html) + "</span>") | join(" "))</td></tr>" ),
        "</tbody></table>",
        "</section>" )
    else empty end ),

  "<section>",
  "<h2>Per connector</h2>",
  "<input id=\"filter\" type=\"search\" placeholder=\"Filter by connector, spec or error text…\" autocomplete=\"off\">",

  ( $d.connectors[]
    | . as $c
    | "<details class=\"conn \(if $c.stats.failed > 0 then "has-fail" else "clean" end)\" \(if $c.stats.failed > 0 then "open" else "" end) data-connector=\"\($c.connector | h)\" id=\"c-\($c.connector | slug)\">",
      "<summary><span class=\"name\">\($c.connector | h)</span>",
      "<span class=\"badge \(if $c.stats.failed > 0 then "bad" else "good" end)\">\($c.stats.failed) failed</span>",
      "<span class=\"meta\">\($c.stats.passed)/\($c.stats.executed) passed (\($c.stats.passPercent)%) · \($c.stats.pending) pending · \($c.stats.skipped) skipped · \($c.specCount) spec(s) · \($c.stats.durationMs | dur)</span>",
      "</summary>",
      ( if ($c.failures | length) == 0 then
          "<p class=\"empty small\">No failures.</p>"
        else
          ( "<table class=\"grid failures\">",
            "<thead><tr><th>Spec</th><th>Test</th><th>Error</th></tr></thead><tbody>",
            ( $c.failures[]
              | "<tr class=\"f\" data-search=\"\((($c.connector + " " + .spec + " " + .fullTitle + " " + (.error.message // "")) | ascii_downcase) | h)\">",
                "<td class=\"spec\"><code>\(.spec | h)</code></td>",
                "<td class=\"test\"><span class=\"suite\">\(.suite | h)</span><span class=\"title\">\(.title | h)</span>",
                ( [ (if .hook then "<span class=\"tag\">hook</span>" else empty end),
                    (if .timedOut then "<span class=\"tag\">timeout</span>" else empty end),
                    (if (.retries // 0) > 0 then "<span class=\"tag\">retry \(.retries)</span>" else empty end),
                    (if (.screenshots // 0) > 0 then "<span class=\"tag\">\(.screenshots) shot(s)</span>" else empty end)
                  ] | join("") ),
                "</td>",
                "<td class=\"err\"><div class=\"msg\">\((.error.message // "no error message") | h)</div>",
                ( if ((.error.stack // "") | length) > 0 then
                    "<details class=\"stack\"><summary>stack</summary><pre>\(.error.stack | h)</pre></details>"
                  else empty end ),
                "</td></tr>" ),
            "</tbody></table>" )
        end ),
      "</details>" ),
  "</section>"
JQ

log "🖨  Rendering HTML…"
{
  cat <<'HTML_HEAD'
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Cypress connector failures</title>
<style>
  :root {
    --bg: #f6f7f9; --panel: #fff; --ink: #14181f; --muted: #5c6672;
    --line: #e2e6ea; --bad: #c02f2f; --bad-bg: #fdeceb; --good: #1c7a44;
    --good-bg: #e8f6ee; --code: #f2f4f7; --accent: #2f5bd0;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #12151a; --panel: #191d24; --ink: #e6e9ee; --muted: #9aa4b1;
      --line: #2a303a; --bad: #ff8b83; --bad-bg: #3a1f1e; --good: #6fd39b;
      --good-bg: #16301f; --code: #21262e; --accent: #8ab0ff;
    }
  }
  * { box-sizing: border-box; }
  body { margin: 0; padding: 24px; background: var(--bg); color: var(--ink);
         font: 14px/1.5 -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; }
  code, pre { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
  .page-head h1 { margin: 0 0 4px; font-size: 22px; }
  .sub { margin: 0 0 20px; color: var(--muted); font-size: 13px; }
  .sub code { background: var(--code); padding: 1px 5px; border-radius: 4px; }
  .cards { display: flex; flex-wrap: wrap; gap: 12px; margin-bottom: 24px; }
  .card { background: var(--panel); border: 1px solid var(--line); border-radius: 10px;
          padding: 12px 18px; min-width: 130px; }
  .card .n { display: block; font-size: 22px; font-weight: 650; }
  .card .l { display: block; color: var(--muted); font-size: 12px; text-transform: uppercase;
             letter-spacing: .04em; }
  .card.bad .n { color: var(--bad); }
  .card.good .n { color: var(--good); }
  h2 { font-size: 15px; text-transform: uppercase; letter-spacing: .05em; color: var(--muted);
       margin: 28px 0 10px; }
  .grid { width: 100%; border-collapse: collapse; background: var(--panel);
          border: 1px solid var(--line); border-radius: 10px; overflow: hidden; }
  .grid th { text-align: left; font-size: 12px; text-transform: uppercase; letter-spacing: .04em;
             color: var(--muted); padding: 8px 12px; border-bottom: 1px solid var(--line); }
  .grid td { padding: 10px 12px; border-bottom: 1px solid var(--line); vertical-align: top; }
  .grid tr:last-child td { border-bottom: 0; }
  .num { font-weight: 650; color: var(--bad); }
  .chip { display: inline-block; background: var(--code); border-radius: 999px;
          padding: 1px 8px; font-size: 12px; margin: 1px 2px 1px 0; }
  #filter { width: 100%; max-width: 460px; padding: 8px 12px; margin-bottom: 14px;
            border: 1px solid var(--line); border-radius: 8px; background: var(--panel);
            color: var(--ink); font-size: 13px; }
  details.conn { background: var(--panel); border: 1px solid var(--line); border-radius: 10px;
                 margin-bottom: 10px; }
  details.conn > summary { cursor: pointer; padding: 12px 14px; display: flex; gap: 10px;
                           align-items: center; flex-wrap: wrap; }
  details.conn .name { font-weight: 650; font-size: 15px; }
  details.conn .meta { color: var(--muted); font-size: 12px; }
  .badge { font-size: 12px; border-radius: 999px; padding: 1px 9px; font-weight: 600; }
  .badge.bad { background: var(--bad-bg); color: var(--bad); }
  .badge.good { background: var(--good-bg); color: var(--good); }
  details.conn .grid { border: 0; border-top: 1px solid var(--line); border-radius: 0; }
  td.spec code { font-size: 12px; color: var(--muted); word-break: break-all; }
  td.test .suite { display: block; font-size: 12px; color: var(--muted); }
  td.test .title { display: block; font-weight: 600; }
  .tag { display: inline-block; margin: 4px 4px 0 0; font-size: 11px; background: var(--code);
         border-radius: 4px; padding: 1px 6px; color: var(--muted); }
  td.err .msg { color: var(--bad); white-space: pre-wrap; word-break: break-word; }
  details.stack { margin-top: 6px; }
  details.stack summary { cursor: pointer; font-size: 12px; color: var(--accent); }
  details.stack pre { background: var(--code); padding: 10px; border-radius: 6px; font-size: 12px;
                      overflow-x: auto; margin: 6px 0 0; }
  .empty { color: var(--good); font-weight: 600; }
  .empty.small { font-weight: 400; color: var(--muted); padding: 0 14px 12px; }
  .hidden { display: none; }
</style>
</head>
<body>
HTML_HEAD

  jq -r "$JQ_HTML" "$JSON_TMP"

  cat <<'HTML_TAIL'
<script>
  (function () {
    var box = document.getElementById("filter");
    if (!box) return;
    box.addEventListener("input", function () {
      var q = box.value.trim().toLowerCase();
      document.querySelectorAll("details.conn").forEach(function (conn) {
        var rows = conn.querySelectorAll("tr.f"), shown = 0;
        rows.forEach(function (row) {
          var hit = !q || row.dataset.search.indexOf(q) !== -1;
          row.classList.toggle("hidden", !hit);
          if (hit) shown++;
        });
        var keep = !q || shown > 0 ||
          conn.dataset.connector.toLowerCase().indexOf(q) !== -1;
        conn.classList.toggle("hidden", !keep);
        if (q && keep) conn.open = true;
      });
    });
  })();
</script>
</body>
</html>
HTML_TAIL
} >"$HTML_TMP"

# Publish both files only once they are complete. mv within the same filesystem is
# atomic, so a concurrent reader sees either the old report or the new one, never a
# half-written mix. Falls back to cp when TMPDIR is on a different filesystem.
mv -f "$JSON_TMP" "$JSON_OUT" 2>/dev/null || cp -f "$JSON_TMP" "$JSON_OUT"
mv -f "$HTML_TMP" "$HTML_OUT" 2>/dev/null || cp -f "$HTML_TMP" "$HTML_OUT"

# ---------------------------------------------------------------------------
# Stage 4 — console summary + exit status.
# ---------------------------------------------------------------------------
jq -r '
  "",
  "──────────── Cypress failure summary ────────────",
  "  connectors      : \(.totals.connectors) (\(.totals.connectorsWithFailures) with failures)",
  "  tests           : \(.totals.tests)  passed \(.totals.passed)  failed \(.totals.failed)  pending \(.totals.pending)  skipped \(.totals.skipped)",
  "  pass rate       : \(.totals.passPercent)% of \(.totals.executed) executed",
  (if (.failingConnectors | length) > 0 then
     "  failing         : \(.failingConnectors | join(", "))" else empty end),
  "─────────────────────────────────────────────────"
' "$JSON_OUT" >&2

log "📄 JSON: ${JSON_OUT}"
log "🌐 HTML: ${HTML_OUT}"

failed_total="$(jq -r '.totals.failed' "$JSON_OUT")"
if [ "$FAIL_ON_FAILURE" = "true" ] && [ "$failed_total" -gt 0 ]; then
  log "❌ ${failed_total} failing test(s) — exiting 1 (--fail-on-failure)"
  exit 1
fi
exit 0
