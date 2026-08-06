import exec from "k6/execution";
import http from "k6/http";
import { Counter } from "k6/metrics";
import { SharedArray } from "k6/data";

const input = new SharedArray("fixture input", () => [JSON.parse(open(__ENV.K6_INPUT))])[0];
const created = new Counter("loadtest_fixture_created");
const failed = new Counter("loadtest_fixture_failed");
const maxDurationSeconds = Math.max(
  60,
  Math.ceil(input.count / Math.max(1, input.concurrency)) * Math.ceil(input.request_timeout_ms / 1000) * 3,
);

export const options = {
  scenarios: {
    fixtures: {
      executor: "shared-iterations",
      exec: "createFixture",
      vus: Math.max(1, input.concurrency),
      iterations: input.count,
      maxDuration: `${maxDurationSeconds}s`,
    },
  },
};

function json(response) {
  try { return response.json(); } catch (_) { return {}; }
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

function post(url, body, headers) {
  return http.post(url, JSON.stringify(body), {
    headers,
    timeout: `${input.request_timeout_ms}ms`,
    redirects: 0,
  });
}

function recordFailure(index, operation, response) {
  failed.add(1, {
    fixture_index: String(index),
    error: `${operation}_${response?.status || "invalid"}`,
  });
}

export function createFixture() {
  const index = exec.scenario.iterationInTest;
  const fixtureId = `fixture_${String(index).padStart(8, "0")}_${input.run_id}`;
  const reference = `customer_loadtest_${input.run_id}_${index}`;
  const routerUrl = input.services.router.replace(/\/$/, "");
  const modularUrl = input.services["modular-pm"].replace(/\/$/, "");
  let customerId = null;
  if (input.plan.requiresCustomer) {
    const response = input.plan.usesPmService
      ? post(`${modularUrl}/v2/customers`, {
        merchant_reference_id: reference,
        name: "Loadtest Modular User",
        phone: "6168205362",
        email: `${reference}@example.com`,
        phone_country_code: "+1",
      }, modularHeaders())
      : post(`${routerUrl}/customers`, {
        customer_id: reference,
        name: "Loadtest User",
        phone: "6168205362",
        email: `${reference}@example.com`,
        phone_country_code: "+1",
      }, apiKeyHeaders());
    const body = json(response);
    customerId = body.id || body.customer_id;
    if (response.status < 200 || response.status >= 300 || !customerId) {
      recordFailure(index, "customer", response);
      return;
    }
  }
  let pmSessionId = null;
  let pmSessionClientSecret = null;
  if (input.plan.usesPmService) {
    const response = post(`${modularUrl}/v2/payment-method-sessions`, {
      ...(customerId ? { customer_id: customerId } : {}),
      expires_in: Number(input.fixture_config.session_expiry || 900),
      storage_type: input.plan.storageType,
    }, modularHeaders());
    const body = json(response);
    pmSessionId = body.id;
    pmSessionClientSecret = body.client_secret;
    if (response.status < 200 || response.status >= 300 || !pmSessionId || !pmSessionClientSecret) {
      recordFailure(index, "pm_session", response);
      return;
    }
  }
  const paymentResponse = post(`${routerUrl}/payments`, {
    amount: Number(input.fixture_config.amount || 1000),
    currency: input.fixture_config.currency || "USD",
    confirm: false,
    capture_method: "automatic",
    profile_id: input.merchant.profile_id,
    session_expiry: Number(input.fixture_config.session_expiry || 900),
    description: `Loadtest automation ${input.plan.id}`,
    ...(customerId ? { customer_id: customerId } : {}),
    ...(input.plan.setupFutureUsage ? { setup_future_usage: input.plan.setupFutureUsage } : {}),
  }, apiKeyHeaders());
  const payment = json(paymentResponse);
  if (paymentResponse.status < 200 || paymentResponse.status >= 300 || !payment.payment_id) {
    recordFailure(index, "payment", paymentResponse);
    return;
  }
  created.add(1, {
    fixture_id: fixtureId,
    payment_id: payment.payment_id,
    customer_id: customerId || "",
    pm_session_id: pmSessionId || "",
    pm_session_client_secret: pmSessionClientSecret || "",
  });
}
