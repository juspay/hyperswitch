import exec from "k6/execution";
import http from "k6/http";
import { Trend } from "k6/metrics";
import { SharedArray } from "k6/data";

const input = new SharedArray("loadtest input", () => [JSON.parse(open(__ENV.K6_INPUT))])[0];
const successStatuses = new Set(["succeeded", "requires_capture", "processing"]);
const pmLatency = new Trend("pm_session_confirm_latency_ms", true);
const paymentLatency = new Trend("payment_confirm_latency_ms", true);
const totalLatency = new Trend("total_latency_ms", true);
const resultLatency = new Trend("loadtest_result_ms", true);

let fixtureOffset = 0;
let phaseOffsetSeconds = 0;
const scenarios = {};
for (const [index, phase] of input.phases.entries()) {
  scenarios[`phase_${index + 1}_${phase.rps}_rps`] = {
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

function modularHeaders(clientSecret) {
  return {
    Authorization: clientSecret
      ? `publishable-key=${input.merchant.publishable_key},client-secret=${clientSecret}`
      : `api-key=${input.merchant.merchant_api_key}`,
    "x-profile-id": input.merchant.profile_id,
    "x-feature": "sandbox-pm-loadtest",
    "content-type": "application/json",
  };
}

function apiKeyHeaders() {
  return { "api-key": input.merchant.merchant_api_key, "content-type": "application/json" };
}

function emitResult(fixture, status, error, pmResponse, paymentResponse, pmMs, paymentMs) {
  const totalMs = pmMs + paymentMs;
  const tags = {
    fixture_id: fixture.fixture_id,
    payment_id: fixture.payment_id,
    result_status: status,
    error: error || "",
    pm_request_id: pmResponse ? requestId(pmResponse) : "",
    payment_request_id: paymentResponse ? requestId(paymentResponse) : "",
    pm_latency_ms: pmMs ? pmMs.toFixed(2) : "",
    payment_latency_ms: paymentMs.toFixed(2),
  };
  if (pmMs > 0) pmLatency.add(pmMs);
  paymentLatency.add(paymentMs);
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
    pmResponse = http.post(
      `${input.services["modular-pm"].replace(/\/$/, "")}/v2/payment-method-sessions/${fixture.pm_session_id}/confirm`,
      JSON.stringify({
        payment_method_data: { card: input.card },
        payment_method_type: "card",
        payment_method_subtype: "credit",
      }),
      { ...params, headers: modularHeaders(fixture.pm_session_client_secret), tags: { operation: "pm_session_confirm" } },
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
    : { payment_method: "card", payment_method_type: "credit", payment_method_data: { card: input.card } };
  if (input.plan.setupFutureUsage) {
    body.setup_future_usage = input.plan.setupFutureUsage;
    body.customer_acceptance = {
      acceptance_type: "online",
      accepted_at: new Date().toISOString(),
      online: { ip_address: "127.0.0.1", user_agent: "k6-loadtest-automation" },
    };
  }
  const paymentResponse = http.post(
    `${input.services.router.replace(/\/$/, "")}/payments/${fixture.payment_id}/confirm`,
    JSON.stringify(body),
    { ...params, headers: apiKeyHeaders(), tags: { operation: "payment_confirm" } },
  );
  const paymentMs = paymentResponse.timings.duration;
  const paymentBody = json(paymentResponse);
  const status = paymentBody.status || "failed";
  const success = paymentResponse.status >= 200 && paymentResponse.status < 300 && successStatuses.has(status);
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
