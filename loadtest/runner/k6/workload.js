import exec from "k6/execution";
import http from "k6/http";
import { Trend } from "k6/metrics";
import { SharedArray } from "k6/data";
import { apiKeyHeaders, modularHeaders } from "./headers.js";

const input = new SharedArray("loadtest input", () => [JSON.parse(open(__ENV.K6_INPUT))])[0];
const successStatuses = new Set(["succeeded", "requires_capture", "processing"]);
const pmLatency = new Trend("pm_session_confirm_latency_ms", true);
const paymentLatency = new Trend("payment_confirm_latency_ms", true);
const hyperswitchInternalLatency = new Trend("hyperswitch_internal_latency_ms", true);
const combinedInternalLatency = new Trend("combined_internal_latency_ms", true);
const totalLatency = new Trend("total_latency_ms", true);
const resultLatency = new Trend("loadtest_result_ms", true);

let fixtureOffset = 0;
let phaseOffsetSeconds = 0;
const scenarios = {};
for (const [index, phase] of input.phases.entries()) {
  const phaseName = `phase_${index + 1}_${phase.rps}_rps`;
  scenarios[phaseName] = {
    executor: "constant-arrival-rate",
    exec: "confirmPayment",
    rate: phase.rps,
    timeUnit: "1s",
    duration: `${phase.holdSeconds}s`,
    startTime: `${phaseOffsetSeconds}s`,
    preAllocatedVUs: Math.max(1, phase.rps * 2),
    maxVUs: Math.max(1, phase.rps * 4),
    gracefulStop: `${Math.ceil(input.request_timeout_ms / 1000)}s`,
    env: { FIXTURE_OFFSET: String(fixtureOffset), FIXTURE_COUNT: String(phase.requests) },
    tags: { rps_phase: phaseName, phase_rps: String(phase.rps) },
  };
  fixtureOffset += phase.requests;
  phaseOffsetSeconds += phase.holdSeconds + phase.idleSeconds;
}

export const options = {
  scenarios,
  summaryTrendStats: ["count", "avg", "min", "med", "p(75)", "p(90)", "p(99)", "max"],
};

function json(response) {
  try { return response.json(); } catch (_) { return {}; }
}

function requestId(response) {
  return response.headers["X-Request-Id"] || response.headers["x-request-id"] || "";
}

function hyperswitchInternalLatencyMs(response) {
  const value = response.headers["X-Hs-Latency"] || response.headers["x-hs-latency"];
  const latency = Number(value);
  return Number.isFinite(latency) ? latency : null;
}

function phaseMetadata() {
  const name = exec.scenario.name;
  const phaseIndex = input.phases.findIndex((phase, index) => name === `phase_${index + 1}_${phase.rps}_rps`);
  const phase = input.phases[phaseIndex];
  return {
    phase: name,
    phase_rps: phase ? String(phase.rps) : "",
  };
}

function measuredCard() {
  if (!input.plan.metadataChanged) return input.card;
  return {
    ...input.card,
    card_exp_month: input.metadata_update.card_exp_month,
    card_exp_year: input.metadata_update.card_exp_year,
    card_holder_name: input.metadata_update.card_holder_name,
  };
}

function customerAcceptance() {
  return {
    acceptance_type: "online",
    accepted_at: new Date().toISOString(),
    online: { ip_address: "127.0.0.1", user_agent: "k6-loadtest-automation" },
  };
}

function emitResult(fixture, status, error, pmResponse, paymentResponse, pmMs, paymentMs) {
  const totalMs = pmMs + paymentMs;
  const internalMs = paymentResponse ? hyperswitchInternalLatencyMs(paymentResponse) : null;
  const combinedInternalMs = internalMs === null ? null : pmMs + internalMs;
  const tags = {
    fixture_id: fixture.fixture_id,
    payment_id: fixture.payment_id,
    result_status: status,
    error: error || "",
    pm_request_id: pmResponse ? requestId(pmResponse) : "",
    payment_request_id: paymentResponse ? requestId(paymentResponse) : "",
    pm_latency_ms: pmMs ? pmMs.toFixed(2) : "",
    payment_latency_ms: paymentMs.toFixed(2),
    hyperswitch_internal_latency_ms: internalMs === null ? "" : internalMs.toFixed(2),
    combined_internal_latency_ms: combinedInternalMs === null ? "" : combinedInternalMs.toFixed(2),
    ...phaseMetadata(),
  };
  if (pmMs > 0) pmLatency.add(pmMs);
  paymentLatency.add(paymentMs);
  if (internalMs !== null) hyperswitchInternalLatency.add(internalMs);
  if (combinedInternalMs !== null) combinedInternalLatency.add(combinedInternalMs);
  totalLatency.add(totalMs);
  resultLatency.add(totalMs, tags);
}

export function confirmPayment() {
  const offset = Number(__ENV.FIXTURE_OFFSET);
  const phaseCount = Number(__ENV.FIXTURE_COUNT);
  const phaseIndex = exec.scenario.iterationInTest;
  if (phaseIndex >= phaseCount) return;
  const fixture = input.fixtures[offset + phaseIndex];
  if (!fixture) return;
  const params = { timeout: `${input.request_timeout_ms}ms`, redirects: 0 };
  let token = null;
  let pmResponse = null;
  let pmMs = 0;
  if (input.plan.usesPmService) {
    const pmSessionConfirmBody = {
      payment_method_data: { card: input.card },
      payment_method_type: "card",
      payment_method_subtype: "credit",
    };
    // PM Modular persists a session-backed card only after recording the
    // customer's acceptance. Guest sessions intentionally remain volatile.
    if (input.plan.setupFutureUsage) {
      pmSessionConfirmBody.customer_acceptance = customerAcceptance();
    }
    pmResponse = http.post(
      `${input.services["modular-pm"].replace(/\/$/, "")}/payment-method-sessions/${fixture.pm_session_id}/confirm`,
      JSON.stringify(pmSessionConfirmBody),
      { ...params, headers: modularHeaders(input, fixture.pm_session_client_secret), tags: { operation: "pm_session_confirm" } },
    );
    pmMs = pmResponse.timings.duration;
    const pmBody = json(pmResponse);
    token = pmBody.associated_payment_methods?.[0]?.payment_method_token;
    if (token && typeof token === "object") token = token.data;
    if (pmResponse.status < 200 || pmResponse.status >= 300 || !token) {
      emitResult(fixture, "failed", `pm_confirm_${pmResponse.status || "invalid"}`, pmResponse, null, pmMs, 0);
      return;
    }
  }
  const body = input.plan.usesPmService
    ? { payment_token: token, payment_method: "card", payment_method_type: "credit" }
    : { payment_method: "card", payment_method_type: "credit", payment_method_data: { card: measuredCard() } };
  if (input.plan.setupFutureUsage) {
    body.setup_future_usage = input.plan.setupFutureUsage;
    body.customer_acceptance = customerAcceptance();
  }
  const paymentResponse = http.post(
    `${input.services.router.replace(/\/$/, "")}/payments/${fixture.payment_id}/confirm`,
    JSON.stringify(body),
    { ...params, headers: apiKeyHeaders(input), tags: { operation: "payment_confirm" } },
  );
  const paymentMs = paymentResponse.timings.duration;
  const paymentBody = json(paymentResponse);
  const status = paymentBody.status || "failed";
  const success = paymentResponse.status >= 200 && paymentResponse.status < 300
    && successStatuses.has(status);
  emitResult(
    fixture,
    success ? status : "failed",
    success ? "" : `payment_confirm_${paymentResponse.status || "invalid"}`,
    pmResponse,
    paymentResponse,
    pmMs,
    paymentMs,
  );
}
