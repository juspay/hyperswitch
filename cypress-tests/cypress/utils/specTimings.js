/* eslint-disable no-console */
/**
 * Per-spec wall-clock accounting for `cypress run`.
 *
 * Cypress's own footer ("All specs passed! 04:16") sums only the mocha run
 * inside the browser, so on a long suite most of the step's wall clock is
 * invisible: a 89-spec connector reported 04:16 against 1118s of real time.
 * The missing time is per-spec, not per-test, and splits into buckets nobody
 * can see from the default output.
 *
 * Every spec is timed into four buckets:
 *
 *   between  time from the previous spec ending to this one starting —
 *            browser relaunch plus the preprocessor re-bundling `support/e2e.js`
 *            (583 KB of commands + redirection handling) for this spec
 *   tests    sum of the test bodies, i.e. what the Cypress footer counts
 *   hooks    the rest of the in-browser mocha run: before/beforeEach/after
 *   spec     the rest of `before:spec` -> `after:spec`: browser boot, support
 *            file evaluation, video finalisation, reporter write
 *
 * `after:run` prints the totals and the worst offenders to stdout, so a CI job
 * log answers "where did the time go" without downloading an artifact.
 */

const MS_PER_SECOND = 1000;
// Enough rows to spot a pattern, few enough to stay readable in a CI log.
const SLOWEST_SPECS_SHOWN = 15;
// Widest spec name in the suite is ~40 chars; columns hold "1234.5s".
const NAME_WIDTH = 42;
const VALUE_WIDTH = 9;

function formatSeconds(ms) {
  return `${(ms / MS_PER_SECOND).toFixed(1)}s`;
}

function formatPercent(part, whole) {
  if (!whole) return "0%";
  return `${((part / whole) * 100).toFixed(1)}%`;
}

/**
 * Sum the test bodies for a spec.
 *
 * The shape of `results.tests[]` has moved across Cypress majors — a duration
 * on the test, on the last attempt, or (with retries) spread over attempts — so
 * every shape is tried and a spec we cannot read contributes 0 rather than
 * throwing mid-run. A zero here widens `hooks`, which the report flags.
 */
function sumTestDurations(results) {
  const tests = results?.tests;
  if (!Array.isArray(tests)) return 0;

  return tests.reduce((total, test) => {
    if (typeof test?.duration === "number") return total + test.duration;

    const attempts = Array.isArray(test?.attempts) ? test.attempts : [];
    const attemptTotal = attempts.reduce(
      (sum, attempt) =>
        sum +
        (typeof attempt?.duration === "number"
          ? attempt.duration
          : (attempt?.wallClockDuration ?? 0)),
      0
    );
    return total + attemptTotal;
  }, 0);
}

/** In-browser mocha duration for the spec, across Cypress stat shapes. */
function mochaDuration(results) {
  const stats = results?.stats;
  if (!stats) return 0;
  if (typeof stats.duration === "number") return stats.duration;
  if (typeof stats.wallClockDuration === "number")
    return stats.wallClockDuration;
  return 0;
}

function printReport(timings) {
  if (timings.length === 0) return;

  const totals = timings.reduce(
    (acc, timing) => ({
      between: acc.between + timing.between,
      tests: acc.tests + timing.tests,
      hooks: acc.hooks + timing.hooks,
      spec: acc.spec + timing.spec,
    }),
    { between: 0, tests: 0, hooks: 0, spec: 0 }
  );
  const wallClock = totals.between + totals.tests + totals.hooks + totals.spec;

  const bucketRow = (label, ms) =>
    `    ${label.padEnd(NAME_WIDTH - 2)}${formatSeconds(ms).padStart(VALUE_WIDTH)}${formatPercent(ms, wallClock).padStart(VALUE_WIDTH)}`;

  console.log("");
  console.log(`  (Spec Timings — ${timings.length} specs)`);
  console.log("");
  console.log(
    `  ${"Wall clock".padEnd(NAME_WIDTH)}${formatSeconds(wallClock).padStart(VALUE_WIDTH)}`
  );
  console.log(bucketRow("tests    (mocha bodies)", totals.tests));
  console.log(bucketRow("hooks    (before/after)", totals.hooks));
  console.log(bucketRow("between  (relaunch+bundle)", totals.between));
  console.log(bucketRow("spec     (boot+video+report)", totals.spec));
  console.log("");

  const overheadPerSpec =
    (totals.between + totals.spec) / timings.length / MS_PER_SECOND;
  console.log(
    `  Fixed cost per spec file: ${overheadPerSpec.toFixed(1)}s  (${timings.length} specs)`
  );
  console.log("");

  const slowest = [...timings]
    .sort((a, b) => b.total - a.total)
    .slice(0, SLOWEST_SPECS_SHOWN);

  const specRow = (name, total, tests, hooks, between, spec) =>
    `  ${name.slice(0, NAME_WIDTH).padEnd(NAME_WIDTH)}` +
    [total, tests, hooks, between, spec]
      .map((value) => value.padStart(VALUE_WIDTH))
      .join("");

  console.log(`  Slowest ${slowest.length} specs`);
  console.log(specRow("", "total", "tests", "hooks", "between", "spec"));
  for (const timing of slowest) {
    console.log(
      specRow(
        timing.name,
        formatSeconds(timing.total),
        formatSeconds(timing.tests),
        formatSeconds(timing.hooks),
        formatSeconds(timing.between),
        formatSeconds(timing.spec)
      )
    );
  }
  console.log("");
}

/**
 * Wire the timing hooks onto Cypress's plugin event bus.
 *
 * Returns the collected timings so a caller can assert on them; the report is
 * printed from `after:run`.
 */
export function registerSpecTimings(on) {
  const timings = [];
  let specStartedAt = null;
  let previousSpecEndedAt = null;

  on("before:spec", () => {
    specStartedAt = Date.now();
  });

  on("after:spec", (spec, results) => {
    const endedAt = Date.now();
    // A crashed spec can reach `after:spec` without its `before:spec` partner.
    if (specStartedAt === null) return;

    const wall = endedAt - specStartedAt;
    const between =
      previousSpecEndedAt === null ? 0 : specStartedAt - previousSpecEndedAt;
    const mocha = mochaDuration(results);
    const tests = Math.min(sumTestDurations(results), mocha);
    const hooks = Math.max(0, mocha - tests);
    // Whatever the browser was doing outside the mocha run.
    const outsideMocha = Math.max(0, wall - mocha);

    timings.push({
      name: spec?.relative ?? spec?.name ?? "unknown",
      // Kept as the sum of the buckets so a printed row always adds up.
      total: between + tests + hooks + outsideMocha,
      between,
      tests,
      hooks,
      spec: outsideMocha,
    });

    previousSpecEndedAt = endedAt;
    specStartedAt = null;
  });

  on("after:run", () => {
    // Diagnostics must never be the reason a green run reports failure.
    try {
      printReport(timings);
    } catch (error) {
      console.log(`  (Spec Timings unavailable: ${error.message})`);
    }
  });

  return timings;
}
