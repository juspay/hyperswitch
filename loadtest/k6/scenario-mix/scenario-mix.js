/**
 * scenario-mix.js — a single k6 load test that mixes all payment scenarios
 * with configurable traffic percentages (example: 50% guest, 40% CIT
 * off-session, 10% metadata-changed).
 *
 * Scenario catalogue and request semantics mirror loadtest/runner (see
 * loadtest/runner/lib/scenarios.js and loadtest/runner/k6/*.js), but unlike
 * the runner there is no separate fixture stage: every iteration is
 * self-contained — it creates everything it needs (customer, payment-method
 * session, saved card, payment) and times each step individually.
 *
 * Usage:
 *   cp config.example.json config.json   # fill in endpoints + credentials
 *   k6 run scenario-mix.js
 * or:
 *   SCENARIO_MIX_CONFIG=/path/to/config.json k6 run scenario-mix.js
 *
 * Load shape: either flat (load.total_rps for load.duration_seconds) or a
 * stepped ramp (load.phases, same knobs as loadtest/runner's loadPhases) that
 * holds each RPS level and reports per-phase percentiles — useful for finding
 * the application's maximum sustainable RPS.
 */
import http from "k6/http";
import { sleep } from "k6";
import { Counter, Trend } from "k6/metrics";
import exec from "k6/execution";
import encoding from "k6/encoding";

// ---------------------------------------------------------------------------
// Configuration loading and validation (fails fast, before any traffic)
// ---------------------------------------------------------------------------

const configPath = __ENV.SCENARIO_MIX_CONFIG || "./config.json";
// Directory prefix for every file this script writes itself (SUMMARY_OUTPUT).
// Defaults to "output" (checked into the repo alongside this script) rather
// than cwd, so a plain `k6 run scenario-mix.js` doesn't scatter files loose
// next to the source. k6 VU/init code can't create directories — the actual
// file write happens in k6's Go runtime after handleSummary returns, and it
// fails outright if the directory doesn't already exist — so a non-default
// OUTPUT_DIR must be created ahead of time (`mkdir -p "$OUTPUT_DIR"`) before
// running. Same constraint applies to any path passed to --console-output or
// --out web-dashboard=export=..., which are k6 CLI flags this script has no
// control over; see README "Collecting everything into one output directory".
const outputDir = __ENV.OUTPUT_DIR || "output";

function joinPath(dir, file) {
  if (!dir) return file;
  return dir.endsWith("/") ? `${dir}${file}` : `${dir}/${file}`;
}

function fail(message) {
  throw new Error(`scenario-mix: ${message}`);
}

let config;
try {
  config = JSON.parse(open(configPath));
} catch (error) {
  fail(`cannot read config at "${configPath}" (set SCENARIO_MIX_CONFIG): ${error}`);
}

const MERCHANT_PATHS = new Set(["non_modular", "modular"]);

// Behavior table ported from loadtest/runner/lib/scenarios.js. Keep in sync.
const SCENARIOS = {
  // No customer and no saved card; modular sessions use volatile storage.
  guest: { requiresCustomer: false, setupFutureUsage: null, storageType: "volatile" },
  // Customer-initiated save-card flows, vaulted inline at confirm.
  cit_on_session: { requiresCustomer: true, setupFutureUsage: "on_session", storageType: "persistent" },
  cit_off_session: { requiresCustomer: true, setupFutureUsage: "off_session", storageType: "persistent" },
  // Payment-to-vault: the session stays volatile, acceptance is recorded, and
  // the card is promoted to persistent storage after authorization.
  ptv_on_session: { requiresCustomer: true, setupFutureUsage: "on_session", storageType: "volatile", modularOnly: true },
  ptv_off_session: { requiresCustomer: true, setupFutureUsage: "off_session", storageType: "volatile", modularOnly: true },
  // Saves a card in the iteration's baseline step, then the measured confirm
  // resubmits the same PAN with changed metadata (expiry/holder name) to
  // exercise vault metadata replacement. Non-modular only.
  cit_metadata_changed: { requiresCustomer: true, requiresSavedCard: true, metadataChanged: true, setupFutureUsage: "off_session", storageType: "persistent" },
  // Replicates the Hyperswitch SDK's pre-confirm calls (payment method list,
  // wallet session tokens, BIN eligibility check) ahead of a guest-style
  // confirm. Router (non_modular) only — the v2/modular payment-method
  // service does not expose 1:1 equivalents of these endpoints yet.
  sdk_checkout: { requiresCustomer: false, setupFutureUsage: null, storageType: "volatile", nonModularOnly: true, sdkFlow: true },
};

// Payment statuses considered a successful measured confirm (workload.js).
const SUCCESS_STATUSES = new Set(["succeeded", "requires_capture", "processing"]);

function numberOr(value, fallback) {
  if (value === undefined || value === null) return fallback;
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) fail(`expected a number, got "${value}"`);
  return parsed;
}

function statusCode(response) {
  return response.status || "invalid";
}

function buildPlan(entry, index) {
  if (!entry || typeof entry !== "object") fail(`scenarios[${index}] must be an object`);
  const name = String(entry.name || "");
  if (!/^[A-Za-z0-9_]+$/.test(name)) {
    fail(`scenarios[${index}]: "name" must be non-empty and use only letters, digits and underscores (it becomes part of metric names), got "${entry.name}"`);
  }
  const merchantPath = String(entry.merchant_path || "").toLowerCase();
  if (!MERCHANT_PATHS.has(merchantPath)) {
    fail(`scenario "${name}": merchant_path must be non_modular or modular, got "${entry.merchant_path}"`);
  }
  const scenarioName = String(entry.scenario || "").toLowerCase();
  const scenario = SCENARIOS[scenarioName];
  if (!scenario) {
    fail(`scenario "${name}": unknown scenario "${entry.scenario}" (known: ${Object.keys(SCENARIOS).join(", ")})`);
  }
  if (scenario.metadataChanged && merchantPath !== "non_modular") {
    fail(`scenario "${name}": ${scenarioName} supports only the non_modular merchant path`);
  }
  if (scenario.modularOnly && merchantPath !== "modular") {
    fail(`scenario "${name}": ${scenarioName} supports only the modular merchant path`);
  }
  if (scenario.nonModularOnly && merchantPath !== "non_modular") {
    fail(`scenario "${name}": ${scenarioName} supports only the non_modular merchant path`);
  }
  const weight = numberOr(entry.weight, NaN);
  if (!(weight >= 0)) fail(`scenario "${name}": weight must be a number >= 0`);
  return {
    name,
    weight,
    merchantPath,
    scenarioName,
    usesPmService: merchantPath === "modular",
    ...scenario,
  };
}

const load = config.load || {};
const paymentConfig = config.payment || {};

const requestTimeoutMs = numberOr(load.request_timeout_ms, 30000);
if (!(requestTimeoutMs >= 100)) fail("load.request_timeout_ms must be >= 100");
const thinkTimeMs = numberOr(load.think_time_ms, 0);
if (thinkTimeMs < 0) fail("load.think_time_ms must be >= 0");
const preAllocatedVusMultiplier = numberOr(load.pre_allocated_vus_multiplier, 3);
// Each iteration is a multi-request flow (up to ~6 sequential requests) and
// flow duration balloons as the system saturates — exactly when VU headroom
// matters, so keep the ceiling generous. It is a ceiling: VUs beyond
// preAllocatedVUs are only used when needed.
const maxVusMultiplier = numberOr(load.max_vus_multiplier, 10);
if (!(maxVusMultiplier >= preAllocatedVusMultiplier && preAllocatedVusMultiplier > 0)) {
  fail("load VU multipliers must satisfy 0 < pre_allocated_vus_multiplier <= max_vus_multiplier");
}

// Flat mode: fixed load.total_rps for load.duration_seconds.
// Ramp mode: load.phases schedules increasing RPS levels, each held long
// enough to reach steady state (same semantics as loadtest/runner's
// loadPhases), and the summary reports per-phase percentiles — the workflow
// for finding the maximum sustainable RPS.
let phaseSchedule = null;
let flatTotalRps = null;
let flatDurationSeconds = null;
if (load.phases && typeof load.phases === "object") {
  if (load.total_rps !== undefined || load.duration_seconds !== undefined) {
    fail("load.total_rps and load.duration_seconds cannot be combined with load.phases");
  }
  const startRps = numberOr(load.phases.starting_rps, NaN);
  if (!(startRps > 0)) fail("load.phases.starting_rps must be a number > 0");
  const targetRps = numberOr(load.phases.target_rps, startRps);
  const stepRps = numberOr(load.phases.step_rps, 0);
  const holdSeconds = numberOr(load.phases.hold_seconds, NaN);
  if (!(holdSeconds >= 1)) fail("load.phases.hold_seconds must be >= 1");
  const idleSeconds = numberOr(load.phases.idle_seconds, 0);
  if (idleSeconds < 0) fail("load.phases.idle_seconds must be >= 0");
  if (startRps > targetRps) fail("load.phases.starting_rps cannot exceed load.phases.target_rps");
  if (startRps < targetRps && stepRps <= 0) {
    fail("load.phases.step_rps must be > 0 when target_rps exceeds starting_rps");
  }
  phaseSchedule = [];
  for (let rps = startRps; ; rps = Math.min(targetRps, rps + stepRps)) {
    phaseSchedule.push({ rps, holdSeconds, idleSeconds });
    if (rps >= targetRps) break;
  }
} else {
  flatTotalRps = numberOr(load.total_rps, NaN);
  if (!(flatTotalRps > 0)) {
    fail("load.total_rps must be a number > 0 (or configure load.phases for a stepped ramp)");
  }
  flatDurationSeconds = numberOr(load.duration_seconds, NaN);
  if (!(flatDurationSeconds >= 1)) fail("load.duration_seconds must be >= 1");
}

const loadDescription = phaseSchedule
  ? `phases: ${phaseSchedule.map((phase) => phase.rps).join(" -> ")} total rps x ${phaseSchedule[0].holdSeconds}s hold + ${phaseSchedule[0].idleSeconds}s idle (${phaseSchedule.length} phases)`
  : `total_rps=${flatTotalRps} | duration=${flatDurationSeconds}s`;

const entries = Array.isArray(config.scenarios) ? config.scenarios : [];
if (!entries.length) fail("scenarios must contain at least one entry");
const plans = entries.map(buildPlan);
const seenNames = new Set();
for (const plan of plans) {
  if (seenNames.has(plan.name)) fail(`duplicate scenario name "${plan.name}"`);
  seenNames.add(plan.name);
}
const weightSum = plans.reduce((sum, plan) => sum + plan.weight, 0);
if (Math.abs(weightSum - 100) > 1e-6) {
  fail(`scenario weights must add up to 100 (got ${weightSum})`);
}
const enabledPlans = plans.filter((plan) => plan.weight > 0);
if (!enabledPlans.length) fail("at least one scenario must have weight > 0");

const services = config.services || {};
const targetHeaders = config.target_headers || {};
const merchant = config.merchant || {};

if (!services.router) fail("services.router is required");
const routerUrl = services.router.replace(/\/$/, "");
// Customers are created through the payment-method service even for
// non-modular CIT scenarios, matching the runner's fixture stage.
const needsModularPm = enabledPlans.some((plan) => plan.usesPmService || plan.requiresCustomer);
// The modular path needs it for payment-method-session confirm auth; sdk_checkout
// needs it to build the SDK Authorization header (see sdkAuthorizationHeader()).
const needsPublishableKey = enabledPlans.some((plan) => plan.usesPmService || plan.sdkFlow);
let modularPmUrl = null;
if (needsModularPm) {
  if (!services.modular_pm) {
    fail("services.modular_pm is required: an enabled scenario uses the modular path or creates customers through the payment-method service");
  }
  modularPmUrl = services.modular_pm.replace(/\/$/, "");
}
if (!merchant.api_key) fail("merchant.api_key is required");
if (!merchant.profile_id) fail("merchant.profile_id is required (set on every payment create)");
if (needsPublishableKey && !merchant.publishable_key) {
  fail("merchant.publishable_key is required for the modular merchant path (payment-method session confirm) and for sdk_checkout (SDK Authorization header)");
}

if (!paymentConfig.card || !paymentConfig.card.card_number) fail("payment.card with card_number is required");
const cardPool = Array.isArray(paymentConfig.card_pool) && paymentConfig.card_pool.length
  ? paymentConfig.card_pool
  : [paymentConfig.card];
for (const [index, card] of cardPool.entries()) {
  if (!card || !card.card_number) fail(`payment.card_pool[${index}] requires card_number`);
}
const metadataChangedUsed = enabledPlans.some((plan) => plan.metadataChanged);
if (metadataChangedUsed && (!paymentConfig.metadata_update || typeof paymentConfig.metadata_update !== "object")) {
  fail("payment.metadata_update is required when cit_metadata_changed is enabled");
}
const metadataUpdate = paymentConfig.metadata_update || {};

const paymentAmount = numberOr(paymentConfig.amount, 1000);
const paymentCurrency = paymentConfig.currency || "USD";
const sessionExpiry = numberOr(paymentConfig.session_expiry, 900);

// Wallets requested by sdk_checkout's session call (POST /payments/session_tokens).
// Empty is valid and is the default — most test merchants have no wallet connectors.
const sdkConfig = config.sdk || {};
const sdkWallets = Array.isArray(sdkConfig.wallets) ? sdkConfig.wallets : [];

// ---------------------------------------------------------------------------
// k6 options: one constant-arrival-rate executor per traffic entry. The split
// is deterministic: rate = total_rps * weight / 100.
// ---------------------------------------------------------------------------

const scenarios = {};
const planByName = {};

function rateFor(plan, phaseRps) {
  return (phaseRps * plan.weight) / 100;
}

// constant-arrival-rate requires an integer iteration count; fractional
// rates are expressed with a longer timeUnit (0.5/s -> 1 iteration / 2s).
function arrivalRate(ratePerSecond) {
  const rounded = Math.round(ratePerSecond * 1e6) / 1e6;
  for (let timeUnitSeconds = 1; timeUnitSeconds <= 36000; timeUnitSeconds += 1) {
    const iterations = rounded * timeUnitSeconds;
    const nearest = Math.round(iterations);
    if (nearest >= 1 && Math.abs(iterations - nearest) < 1e-9) {
      return { rate: nearest, timeUnit: `${timeUnitSeconds}s` };
    }
  }
  fail(`cannot express rate ${ratePerSecond}/s as an integer arrival rate`);
}

function executor(name, plan, ratePerSecond, durationSeconds, startTimeSeconds, phaseIndex) {
  const arrival = arrivalRate(ratePerSecond);
  const preAllocatedVUs = Math.max(1, Math.ceil(ratePerSecond * preAllocatedVusMultiplier));
  scenarios[name] = {
    executor: "constant-arrival-rate",
    exec: "runScenario",
    rate: arrival.rate,
    timeUnit: arrival.timeUnit,
    duration: `${durationSeconds}s`,
    preAllocatedVUs,
    maxVUs: Math.max(preAllocatedVUs, Math.ceil(ratePerSecond * maxVusMultiplier)),
    // An iteration runs several sequential requests; allow ramp-down to cover
    // roughly a worst-case full flow.
    gracefulStop: `${Math.ceil((requestTimeoutMs * 8) / 1000)}s`,
    env: phaseIndex === null
      ? { SCENARIO_NAME: plan.name }
      : { SCENARIO_NAME: plan.name, PHASE_INDEX: String(phaseIndex) },
    tags: phaseIndex === null
      ? { traffic_scenario: plan.name }
      : { traffic_scenario: plan.name, phase: `phase_${phaseIndex + 1}_${phaseSchedule[phaseIndex].rps}_rps` },
  };
  if (startTimeSeconds > 0) scenarios[name].startTime = `${startTimeSeconds}s`;
}

if (phaseSchedule) {
  let startTimeSeconds = 0;
  phaseSchedule.forEach((phase, phaseIndex) => {
    for (const plan of enabledPlans) {
      executor(`${plan.name}_p${phaseIndex + 1}`, plan, rateFor(plan, phase.rps), phase.holdSeconds, startTimeSeconds, phaseIndex);
    }
    startTimeSeconds += phase.holdSeconds + phase.idleSeconds;
  });
} else {
  for (const plan of enabledPlans) {
    executor(plan.name, plan, rateFor(plan, flatTotalRps), flatDurationSeconds, 0, null);
  }
}
for (const plan of enabledPlans) planByName[plan.name] = plan;

export const options = {
  scenarios,
  thresholds: config.thresholds && typeof config.thresholds === "object" ? config.thresholds : {},
  summaryTrendStats: ["count", "avg", "med", "p(75)", "p(90)", "p(99)", "max"],
};

// ---------------------------------------------------------------------------
// Metrics: per-scenario Trends so the summary splits percentiles per traffic
// entry, plus one cross-scenario confirm trend and success/failure counters.
// ---------------------------------------------------------------------------

const globalPaymentConfirm = new Trend("payment_confirm_latency_ms", true);

function stepNames(plan) {
  const steps = [];
  if (plan.requiresCustomer) steps.push("customer_create");
  if (plan.usesPmService) steps.push("pm_session_create");
  if (plan.requiresSavedCard) steps.push("baseline_create", "baseline_confirm");
  steps.push("payment_create");
  if (plan.sdkFlow) steps.push("payment_method_list", "session", "eligibility");
  if (plan.usesPmService) steps.push("pm_session_confirm");
  steps.push("payment_confirm");
  return steps;
}

for (const plan of enabledPlans) {
  plan.trends = {};
  for (const step of stepNames(plan)) {
    plan.trends[step] = new Trend(`${step}_ms_${plan.name}`, true);
  }
  plan.trends.total_flow = new Trend(`total_flow_ms_${plan.name}`, true);
  plan.successCounter = new Counter(`scenario_success_${plan.name}`);
  plan.failureCounter = new Counter(`scenario_failure_${plan.name}`);
  // In ramp mode, payment-confirm latency and success/failure are also
  // tracked per phase (suffix _p<N>) so the summary shows where the
  // saturation knee is. Step trends stay aggregated across phases.
  plan.phases = phaseSchedule
    ? phaseSchedule.map((phase, index) => ({
      index: index + 1,
      rps: phase.rps,
      rate: rateFor(plan, phase.rps),
      confirmTrend: new Trend(`payment_confirm_ms_${plan.name}_p${index + 1}`, true),
      successCounter: new Counter(`scenario_success_${plan.name}_p${index + 1}`),
      failureCounter: new Counter(`scenario_failure_${plan.name}_p${index + 1}`),
    }))
    : null;
}

// ---------------------------------------------------------------------------
// Request helpers (header shapes ported from loadtest/runner/k6/headers.js)
// ---------------------------------------------------------------------------

function json(response) {
  try { return response.json(); } catch (_) { return {}; }
}

function requestParams(headers, operation) {
  return { headers, timeout: `${requestTimeoutMs}ms`, redirects: 0, tags: { operation } };
}

function post(url, body, headers, operation) {
  return http.post(url, JSON.stringify(body), requestParams(headers, operation));
}

function get(url, headers, operation) {
  return http.get(url, requestParams(headers, operation));
}

function apiKeyHeaders() {
  return {
    ...(targetHeaders.router || {}),
    "api-key": merchant.api_key,
    "content-type": "application/json",
  };
}

function modularApiKeyHeaders() {
  return {
    ...(targetHeaders.modular_pm || {}),
    Authorization: `api-key=${merchant.api_key}`,
    "x-profile-id": merchant.profile_id,
    "content-type": "application/json",
  };
}

function modularSessionHeaders(clientSecret) {
  return {
    ...(targetHeaders.modular_pm || {}),
    Authorization: `publishable-key=${merchant.publishable_key},client-secret=${clientSecret}`,
    "x-profile-id": merchant.profile_id,
    "content-type": "application/json",
  };
}

// Base64 (standard, padded) of comma-separated key=value pairs, decoded by
// SdkAuthorization::decode (crates/hyperswitch_domain_models/src/sdk_auth.rs).
// Replicates the Authorization header the Hyperswitch SDK sends on
// payment_method_list / session / eligibility once it holds a payment_id and
// client_secret from payment create.
function sdkAuthorizationHeader(payment) {
  const parts = [
    `profile_id=${merchant.profile_id}`,
    `publishable_key=${merchant.publishable_key}`,
    `client_secret=${payment.client_secret}`,
    `payment_id=${payment.payment_id}`,
  ];
  return encoding.b64encode(parts.join(","));
}

function sdkAuthHeaders(payment) {
  return {
    ...(targetHeaders.router || {}),
    Authorization: sdkAuthorizationHeader(payment),
    "content-type": "application/json",
  };
}

function customerAcceptance() {
  return {
    acceptance_type: "online",
    accepted_at: new Date().toISOString(),
    online: { ip_address: "127.0.0.1", user_agent: "k6-scenario-mix" },
  };
}

function pickCard(iteration) {
  return cardPool[iteration % cardPool.length];
}

function paymentCreateBody(plan, customerId, description) {
  return {
    amount: paymentAmount,
    currency: paymentCurrency,
    confirm: false,
    capture_method: "automatic",
    profile_id: merchant.profile_id,
    session_expiry: sessionExpiry,
    description,
    ...(customerId ? { customer_id: customerId } : {}),
    ...(plan.setupFutureUsage ? { setup_future_usage: plan.setupFutureUsage } : {}),
  };
}

const FAILURE_LOG_BODY_MAX_CHARS = 500;

function truncateBody(body) {
  if (typeof body !== "string") return body;
  return body.length > FAILURE_LOG_BODY_MAX_CHARS
    ? `${body.slice(0, FAILURE_LOG_BODY_MAX_CHARS)}…(truncated)`
    : body;
}

// One JSON record per failed iteration, via console.error. k6 VU code cannot
// write files directly (open() is read-only, init-stage only), so the way to
// get a durable failures file is k6's own console-output redirection: run
// with `k6 run --console-output=failures.log` (or `K6_CONSOLE_OUTPUT=failures.log`)
// and every console.error call below lands there as its own line instead of
// the terminal — see README "Logging failures to a file".
function logFailure(plan, phaseInfo, reason, response, errorMessage) {
  const record = {
    time: new Date().toISOString(),
    scenario: plan.name,
    merchant_path: plan.merchantPath,
    scenario_type: plan.scenarioName,
    phase: phaseInfo ? phaseInfo.index : undefined,
    vu: __VU,
    iteration: exec.scenario.iterationInTest,
    reason,
  };
  if (response) {
    record.status = response.status;
    record.url = response.url;
    record.error = response.error || undefined;
    record.body = truncateBody(response.body);
  }
  if (errorMessage) record.error_message = errorMessage;
  console.error(JSON.stringify(record));
}

function failIteration(plan, startedAt, reason, phaseInfo, response, errorMessage) {
  plan.failureCounter.add(1, { reason });
  if (phaseInfo) phaseInfo.failureCounter.add(1, { reason });
  plan.trends.total_flow.add(Date.now() - startedAt);
  logFailure(plan, phaseInfo, reason, response, errorMessage);
}

// Card persistence can complete shortly after a successful confirm. Poll the
// payment until the saved payment method surfaces (fixtures.js behavior).
function findSavedPaymentMethod(paymentId) {
  for (let attempt = 0; attempt < 50; attempt += 1) {
    const response = http.get(`${routerUrl}/payments/${paymentId}`, requestParams(apiKeyHeaders(), "baseline_poll"));
    const paymentMethodId = json(response).payment_method_id;
    if (paymentMethodId) return paymentMethodId;
    sleep(0.1);
  }
  return null;
}

// ---------------------------------------------------------------------------
// Measured traffic
// ---------------------------------------------------------------------------

export function runScenario() {
  const plan = planByName[__ENV.SCENARIO_NAME];
  const phaseInfo = plan.phases ? plan.phases[Number(__ENV.PHASE_INDEX || 0)] : null;
  const startedAt = Date.now();
  // Never let an unexpected exception kill an iteration invisibly: k6 would
  // count it as complete with no success/failure recorded, silently
  // understating the failure rate.
  try {
    runFlow(plan, phaseInfo, startedAt);
  } catch (error) {
    failIteration(plan, startedAt, `exception_${(error && error.name) || "error"}`, phaseInfo, null, error && error.message);
  }
}

function runFlow(plan, phaseInfo, startedAt) {
  const iteration = exec.scenario.iterationInTest;
  const card = pickCard(iteration);

  // Step 1: customer (CIT and saved-card scenarios only; guests skip this).
  let customerId = null;
  if (plan.requiresCustomer) {
    const reference = `customer_mix_${plan.name}_${iteration}_${__VU}_${Date.now()}`;
    const response = post(
      `${modularPmUrl}/customers`,
      {
        merchant_reference_id: reference,
        name: "Loadtest User",
        phone: "6168205362",
        email: `${reference}@example.com`,
        phone_country_code: "+1",
      },
      modularApiKeyHeaders(),
      "customer_create",
    );
    plan.trends.customer_create.add(response.timings.duration);
    const body = json(response);
    customerId = body.id || body.customer_id;
    if (response.status < 200 || response.status >= 300 || !customerId) {
      failIteration(plan, startedAt, `customer_create_${statusCode(response)}`, phaseInfo, response);
      return;
    }
  }

  // Step 2: payment-method session (modular merchant path only).
  let pmSessionId = null;
  let pmSessionClientSecret = null;
  if (plan.usesPmService) {
    const response = post(
      `${modularPmUrl}/payment-method-sessions`,
      {
        ...(customerId ? { customer_id: customerId } : {}),
        expires_in: sessionExpiry,
        storage_type: plan.storageType,
      },
      modularApiKeyHeaders(),
      "pm_session_create",
    );
    plan.trends.pm_session_create.add(response.timings.duration);
    const body = json(response);
    pmSessionId = body.id;
    pmSessionClientSecret = body.client_secret;
    if (response.status < 200 || response.status >= 300 || !pmSessionId || !pmSessionClientSecret) {
      failIteration(plan, startedAt, `pm_session_create_${statusCode(response)}`, phaseInfo, response);
      return;
    }
  }

  // Step 3: baseline saved card (cit_metadata_changed). The measured confirm
  // later resubmits the same PAN with changed metadata.
  if (plan.requiresSavedCard) {
    const createResponse = post(
      `${routerUrl}/payments`,
      paymentCreateBody(plan, customerId, `scenario-mix baseline ${plan.merchantPath}:${plan.scenarioName}`),
      apiKeyHeaders(),
      "baseline_create",
    );
    plan.trends.baseline_create.add(createResponse.timings.duration);
    const baseline = json(createResponse);
    if (createResponse.status < 200 || createResponse.status >= 300 || !baseline.payment_id) {
      failIteration(plan, startedAt, `baseline_create_${statusCode(createResponse)}`, phaseInfo, createResponse);
      return;
    }
    const confirmResponse = post(
      `${routerUrl}/payments/${baseline.payment_id}/confirm`,
      {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: { card },
        // The baseline must finish storing the card before measured traffic.
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance(),
      },
      apiKeyHeaders(),
      "baseline_confirm",
    );
    plan.trends.baseline_confirm.add(confirmResponse.timings.duration);
    const savedPaymentMethodId = json(confirmResponse).payment_method_id
      || findSavedPaymentMethod(baseline.payment_id);
    if (confirmResponse.status < 200 || confirmResponse.status >= 300 || !savedPaymentMethodId) {
      failIteration(plan, startedAt, `baseline_confirm_${statusCode(confirmResponse)}`, phaseInfo, confirmResponse);
      return;
    }
  }

  // Optional settling pause between preparation and the measured requests.
  if (thinkTimeMs > 0) sleep(thinkTimeMs / 1000);

  // Step 4: create the payment that the measured confirm operates on.
  const createResponse = post(
    `${routerUrl}/payments`,
    paymentCreateBody(plan, customerId, `scenario-mix ${plan.merchantPath}:${plan.scenarioName}`),
    apiKeyHeaders(),
    "payment_create",
  );
  plan.trends.payment_create.add(createResponse.timings.duration);
  const payment = json(createResponse);
  if (createResponse.status < 200 || createResponse.status >= 300 || !payment.payment_id) {
    failIteration(plan, startedAt, `payment_create_${statusCode(createResponse)}`, phaseInfo, createResponse);
    return;
  }

  // Step 4b: SDK pre-confirm calls (sdk_checkout only) — replicates what the
  // Hyperswitch SDK does between intent creation and confirm: list available
  // payment methods, fetch wallet session tokens, and run an eligibility
  // check for the card the shopper is about to submit.
  if (plan.sdkFlow) {
    const pmListResponse = get(
      `${routerUrl}/payments/${payment.payment_id}/client`,
      sdkAuthHeaders(payment),
      "payment_method_list",
    );
    plan.trends.payment_method_list.add(pmListResponse.timings.duration);
    if (pmListResponse.status < 200 || pmListResponse.status >= 300) {
      failIteration(plan, startedAt, `payment_method_list_${statusCode(pmListResponse)}`, phaseInfo, pmListResponse);
      return;
    }

    const sessionResponse = post(
      `${routerUrl}/payments/session_tokens`,
      { payment_id: payment.payment_id, client_secret: payment.client_secret, wallets: sdkWallets },
      sdkAuthHeaders(payment),
      "session",
    );
    plan.trends.session.add(sessionResponse.timings.duration);
    if (sessionResponse.status < 200 || sessionResponse.status >= 300) {
      failIteration(plan, startedAt, `session_${statusCode(sessionResponse)}`, phaseInfo, sessionResponse);
      return;
    }

    const eligibilityResponse = post(
      `${routerUrl}/payments/${payment.payment_id}/eligibility`,
      {
        payment_method_type: "card",
        payment_method_subtype: "credit",
        payment_method_data: { card },
      },
      sdkAuthHeaders(payment),
      "eligibility",
    );
    plan.trends.eligibility.add(eligibilityResponse.timings.duration);
    if (eligibilityResponse.status < 200 || eligibilityResponse.status >= 300) {
      failIteration(plan, startedAt, `eligibility_${statusCode(eligibilityResponse)}`, phaseInfo, eligibilityResponse);
      return;
    }
  }

  // Step 5: confirm the payment-method session to obtain a payment token
  // (modular merchant path only).
  let token = null;
  if (plan.usesPmService) {
    const body = {
      payment_method_data: { card },
      payment_method_type: "card",
      payment_method_subtype: "credit",
    };
    // PM Modular persists a session-backed card only after recording the
    // customer's acceptance. Guest sessions intentionally remain volatile.
    if (plan.setupFutureUsage) body.customer_acceptance = customerAcceptance();
    const response = post(
      `${modularPmUrl}/payment-method-sessions/${pmSessionId}/confirm`,
      body,
      modularSessionHeaders(pmSessionClientSecret),
      "pm_session_confirm",
    );
    plan.trends.pm_session_confirm.add(response.timings.duration);
    const pmBody = json(response);
    token = pmBody.associated_payment_methods?.[0]?.payment_method_token;
    if (token && typeof token === "object") token = token.data;
    if (response.status < 200 || response.status >= 300 || !token) {
      failIteration(plan, startedAt, `pm_session_confirm_${statusCode(response)}`, phaseInfo, response);
      return;
    }
  }

  // Step 6: measured payment confirm.
  const measuredCard = plan.metadataChanged ? { ...card, ...metadataUpdate } : card;
  const confirmBody = plan.usesPmService
    ? { payment_token: token, payment_method: "card", payment_method_type: "credit" }
    : { payment_method: "card", payment_method_type: "credit", payment_method_data: { card: measuredCard } };
  if (plan.setupFutureUsage) {
    confirmBody.setup_future_usage = plan.setupFutureUsage;
    confirmBody.customer_acceptance = customerAcceptance();
  }
  const confirmResponse = post(
    `${routerUrl}/payments/${payment.payment_id}/confirm`,
    confirmBody,
    apiKeyHeaders(),
    "payment_confirm",
  );
  plan.trends.payment_confirm.add(confirmResponse.timings.duration);
  globalPaymentConfirm.add(confirmResponse.timings.duration);
  if (phaseInfo) phaseInfo.confirmTrend.add(confirmResponse.timings.duration);
  const status = json(confirmResponse).status || "failed";
  const succeeded = confirmResponse.status >= 200
    && confirmResponse.status < 300
    && SUCCESS_STATUSES.has(status);
  if (!succeeded) {
    failIteration(plan, startedAt, `payment_confirm_${statusCode(confirmResponse)}_${status}`, phaseInfo, confirmResponse);
    return;
  }
  plan.successCounter.add(1);
  if (phaseInfo) phaseInfo.successCounter.add(1);
  plan.trends.total_flow.add(Date.now() - startedAt);
}

// ---------------------------------------------------------------------------
// Summary: per-scenario percentile table plus optional full JSON export.
// ---------------------------------------------------------------------------

function metricValues(data, name) {
  const metric = data.metrics[name];
  return metric ? metric.values : null;
}

function countOf(values) {
  return values ? values.count : 0;
}

// The custom summary output replaces k6's default end-of-test summary, so it
// must surface the signals that say whether the test itself was valid:
// dropped_iterations > 0 means the client could not sustain the requested
// rate (raise the VU multipliers and rerun — numbers below are unreliable).
function globalSummaryRow(data) {
  const droppedCount = countOf(metricValues(data, "dropped_iterations"));
  const httpFailed = metricValues(data, "http_req_failed");
  const failedRate = httpFailed ? (httpFailed.rate * 100).toFixed(2) : "0.00";
  const warning = droppedCount > 0
    ? "  WARNING: reports at the requested rate were NOT all achieved; raise VU multipliers and rerun"
    : "";
  const peak = peakIterationRateRaw(data);
  const peakStr = peak === null ? "-" : `${peak.toFixed(2)}/s`;
  return `global | iterations=${countOf(metricValues(data, "iterations"))} | peak_iteration_rate=${peakStr} | dropped_iterations=${droppedCount}${warning} | http_reqs=${countOf(metricValues(data, "http_reqs"))} | http_req_failed=${failedRate}%`;
}

// The highest iteration rate (all iterations, success or failure — same
// thing k6's own "Iteration Rate" dashboard tile measures) sustained across
// the run: in ramp mode, the max over phases of (that phase's total
// iterations across every scenario / that phase's hold_seconds); in flat
// mode, the single sustained rate across the whole run. This intentionally
// counts every iteration, not just successes — it answers "how many
// flows/sec did this actually push," distinct from achieved TPS above
// (business-success throughput). It only reflects handleSummary's final
// aggregate, not a live per-second trace — k6 doesn't expose a live-readable
// iteration counter from VU code, and the web dashboard's live/exported
// charts are a separate output plugin this script has no way to add panels
// to (see README), so this can only ever show up in the stdout summary and
// SUMMARY_OUTPUT below, never inside timeseries.html itself.
function peakIterationRateRaw(data) {
  const totalIterationsAt = (suffix) => enabledPlans.reduce((sum, plan) => sum
    + countOf(metricValues(data, `scenario_success_${plan.name}${suffix}`))
    + countOf(metricValues(data, `scenario_failure_${plan.name}${suffix}`)), 0);
  if (!phaseSchedule) {
    return achievedTpsRaw(totalIterationsAt(""), flatDurationSeconds);
  }
  let peak = null;
  phaseSchedule.forEach((phase, index) => {
    const rate = achievedTpsRaw(totalIterationsAt(`_p${index + 1}`), phase.holdSeconds);
    if (rate !== null && (peak === null || rate > peak)) peak = rate;
  });
  return peak;
}

function fmt(values, key) {
  if (!values || values[key] === undefined || values.count === 0) return "-";
  return values[key].toFixed(2);
}

// Achieved TPS is successful transactions / wall-clock window — distinct
// from "target rps" (the offered/requested iteration rate): if any measured
// confirms fail or iterations are dropped, achieved TPS falls below target.
// Raw numeric form (null when the window is unknown) feeds both the text
// table below and the synthetic gauge metrics injected for the HTML report;
// k6's own Counter.rate can't be reused for this because in ramp mode it
// divides by the whole test's wall-clock time, not a single phase's
// hold_seconds.
function achievedTpsRaw(successCount, windowSeconds) {
  return windowSeconds > 0 ? successCount / windowSeconds : null;
}

function achievedTps(successCount, windowSeconds) {
  const raw = achievedTpsRaw(successCount, windowSeconds);
  return raw === null ? "-" : raw.toFixed(2);
}

// SUMMARY_OUTPUT's JSON dump only contains whatever is in data.metrics;
// achieved TPS is a value this script derives (success count / window)
// rather than something k6 tracks natively, so it has no metric to show
// without this. handleSummary's `data` is a plain, still-mutable JS object
// at this point (not yet serialized), so adding entries here is enough for
// JSON.stringify(data) below to pick them up. Shape is k6's gauge format:
// values.value/min/max (min/max just mirror value — there's only one sample).
function injectAchievedTpsMetrics(data) {
  const setGauge = (name, value) => {
    data.metrics[name] = { type: "gauge", contains: "default", values: { value, min: value, max: value } };
  };
  if (phaseSchedule) {
    for (const plan of enabledPlans) {
      for (const phaseInfo of plan.phases) {
        const successValues = metricValues(data, `scenario_success_${plan.name}_p${phaseInfo.index}`);
        const holdSeconds = phaseSchedule[phaseInfo.index - 1].holdSeconds;
        const tps = achievedTpsRaw(countOf(successValues), holdSeconds);
        if (tps !== null) setGauge(`achieved_tps_${plan.name}_p${phaseInfo.index}`, tps);
      }
    }
  } else {
    for (const plan of enabledPlans) {
      const successValues = metricValues(data, `scenario_success_${plan.name}`);
      const tps = achievedTpsRaw(countOf(successValues), flatDurationSeconds);
      if (tps !== null) setGauge(`achieved_tps_${plan.name}`, tps);
    }
  }
  const peak = peakIterationRateRaw(data);
  if (peak !== null) setGauge("peak_iteration_rate", peak);
}

function flatSummaryRow(data, plan) {
  const confirmValues = metricValues(data, `payment_confirm_ms_${plan.name}`);
  const flowValues = metricValues(data, `total_flow_ms_${plan.name}`);
  const successValues = metricValues(data, `scenario_success_${plan.name}`);
  const failureValues = metricValues(data, `scenario_failure_${plan.name}`);
  const successCount = successValues ? successValues.count : 0;
  return [
    plan.name,
    String(plan.weight),
    ((flatTotalRps * plan.weight) / 100).toFixed(2),
    achievedTps(successCount, flatDurationSeconds),
    fmt(confirmValues, "med"),
    fmt(confirmValues, "p(90)"),
    fmt(confirmValues, "p(99)"),
    fmt(flowValues, "med"),
    fmt(flowValues, "p(90)"),
    String(successCount),
    failureValues ? String(failureValues.count) : "0",
  ].join(" | ");
}

function rampSummaryRows(data, plan) {
  const columns = ["phase", "target rps", "achieved tps", "confirm p50", "confirm p90", "confirm p99", "success", "failure"];
  const rows = [
    `\n${plan.name} (weight ${plan.weight}%)`,
    columns.join(" | "),
    columns.map((column) => "-".repeat(column.length)).join(" | "),
  ];
  for (const phaseInfo of plan.phases) {
    const confirmValues = metricValues(data, `payment_confirm_ms_${plan.name}_p${phaseInfo.index}`);
    const successValues = metricValues(data, `scenario_success_${plan.name}_p${phaseInfo.index}`);
    const failureValues = metricValues(data, `scenario_failure_${plan.name}_p${phaseInfo.index}`);
    const successCount = successValues ? successValues.count : 0;
    const holdSeconds = phaseSchedule[phaseInfo.index - 1].holdSeconds;
    rows.push([
      String(phaseInfo.index),
      phaseInfo.rate.toFixed(2),
      achievedTps(successCount, holdSeconds),
      fmt(confirmValues, "med"),
      fmt(confirmValues, "p(90)"),
      fmt(confirmValues, "p(99)"),
      String(successCount),
      failureValues ? String(failureValues.count) : "0",
    ].join(" | "));
  }
  return rows;
}

export function handleSummary(data) {
  injectAchievedTpsMetrics(data);
  const header = `scenario-mix | config=${configPath} | ${loadDescription}`;
  const rows = [header, globalSummaryRow(data)];
  if (phaseSchedule) {
    for (const plan of enabledPlans) rows.push(...rampSummaryRows(data, plan));
  } else {
    const columns = [
      "scenario", "weight%", "target rps", "achieved tps",
      "confirm p50", "confirm p90", "confirm p99",
      "flow p50", "flow p90",
      "success", "failure",
    ];
    rows.push(columns.join(" | "));
    rows.push(columns.map((column) => "-".repeat(column.length)).join(" | "));
    for (const plan of enabledPlans) rows.push(flatSummaryRow(data, plan));
  }
  const output = { stdout: `\n${rows.join("\n")}\n` };
  if (__ENV.SUMMARY_OUTPUT) {
    output[joinPath(outputDir, __ENV.SUMMARY_OUTPUT)] = JSON.stringify(data, null, 2);
  }
  return output;
}
