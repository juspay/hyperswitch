import exec from "k6/execution";
import http from "k6/http";
import { sleep } from "k6";
import { Counter } from "k6/metrics";
import { SharedArray } from "k6/data";
import { apiKeyHeaders, modularHeaders } from "./headers.js";

const input = new SharedArray("fixture input", () => [JSON.parse(open(__ENV.K6_INPUT))])[0];
const created = new Counter("loadtest_fixture_created");
const failed = new Counter("loadtest_fixture_failed");
const fixtureConcurrency = input.plan.metadataChanged ? 1 : Math.max(1, input.concurrency);
const maxDurationSeconds = Math.max(
  60,
  Math.ceil(input.count / fixtureConcurrency) * Math.ceil(input.request_timeout_ms / 1000) * 3,
);

export const options = {
  setupTimeout: `${Math.max(60, Math.ceil(input.request_timeout_ms / 1000) * 4)}s`,
  scenarios: {
    fixtures: {
      executor: "shared-iterations",
      exec: "createFixture",
      // Metadata-change fixtures intentionally reuse one test PAN. Serialize
      // their baseline saves so legacy vault deduplication cannot race.
      vus: fixtureConcurrency,
      iterations: input.count,
      maxDuration: `${maxDurationSeconds}s`,
    },
  },
};

function json(response) {
  try { return response.json(); } catch (_) { return {}; }
}

function post(url, body, headers) {
  return http.post(url, JSON.stringify(body), {
    headers,
    timeout: `${input.request_timeout_ms}ms`,
    redirects: 0,
  });
}

function get(url, headers) {
  return http.get(url, {
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

function customerAcceptance() {
  return {
    acceptance_type: "online",
    accepted_at: new Date().toISOString(),
    online: { ip_address: "127.0.0.1", user_agent: "k6-loadtest-automation" },
  };
}

function createPayment(routerUrl, customerId, description, setupFutureUsage = input.plan.setupFutureUsage) {
  return post(`${routerUrl}/payments`, {
    amount: Number(input.fixture_config.amount || 1000),
    currency: input.fixture_config.currency || "USD",
    confirm: false,
    capture_method: "automatic",
    profile_id: input.merchant.profile_id,
    session_expiry: Number(input.fixture_config.session_expiry || 900),
    description,
    ...(customerId ? { customer_id: customerId } : {}),
    ...(setupFutureUsage ? { setup_future_usage: setupFutureUsage } : {}),
  }, apiKeyHeaders(input));
}

function createCustomer(modularUrl, reference) {
  return post(`${modularUrl}/customers`, {
    merchant_reference_id: reference,
    name: "Loadtest User",
    phone: "6168205362",
    email: `${reference}@example.com`,
    phone_country_code: "+1",
  }, modularHeaders(input));
}

function createPmSession(modularUrl, customerId, storageType = input.plan.storageType) {
  return post(`${modularUrl}/payment-method-sessions`, {
    ...(customerId ? { customer_id: customerId } : {}),
    expires_in: Number(input.fixture_config.session_expiry || 900),
    storage_type: storageType,
  }, modularHeaders(input));
}

function extractToken(value) {
  return typeof value === "object" && value !== null ? value.data : value;
}

function findSavedPaymentMethod(routerUrl, paymentId) {
  // Card persistence can complete shortly after a successful confirm under
  // concurrent fixture creation. Keep this wait outside the measured phase.
  for (let attempt = 0; attempt < 50; attempt += 1) {
    const response = http.get(`${routerUrl}/payments/${paymentId}`, {
      headers: apiKeyHeaders(input),
      timeout: `${input.request_timeout_ms}ms`,
      redirects: 0,
    });
    const paymentMethodId = json(response).payment_method_id;
    if (paymentMethodId) return paymentMethodId;
    sleep(0.1);
  }
  return null;
}

export function setup() {
  if (!input.plan.sharedCustomer) return null;

  const routerUrl = input.services.router.replace(/\/$/, "");
  const modularUrl = input.services["modular-pm"].replace(/\/$/, "");
  const reference = `customer_loadtest_shared_${input.run_id}`;
  const customerResponse = createCustomer(modularUrl, reference);
  const customer = json(customerResponse);
  const customerId = customer.id || customer.customer_id;
  if (customerResponse.status < 200 || customerResponse.status >= 300 || !customerId) {
    throw new Error(`shared customer creation failed: status=${customerResponse.status}`);
  }

  const sessionResponse = createPmSession(modularUrl, customerId, "persistent");
  const session = json(sessionResponse);
  if (
    sessionResponse.status < 200
    || sessionResponse.status >= 300
    || !session.id
    || !session.client_secret
  ) {
    throw new Error(`shared card session creation failed: status=${sessionResponse.status}`);
  }

  const sessionConfirmResponse = post(
    `${modularUrl}/payment-method-sessions/${session.id}/confirm`,
    {
      payment_method_data: { card: input.card },
      payment_method_type: "card",
      payment_method_subtype: "credit",
      customer_acceptance: customerAcceptance(),
    },
    modularHeaders(input, session.client_secret),
  );
  const sessionConfirm = json(sessionConfirmResponse);
  const token = extractToken(sessionConfirm.associated_payment_methods?.[0]?.payment_method_token);
  if (sessionConfirmResponse.status < 200 || sessionConfirmResponse.status >= 300 || !token) {
    throw new Error(`shared card session confirmation failed: status=${sessionConfirmResponse.status}`);
  }

  const baselinePaymentResponse = createPayment(
    routerUrl,
    customerId,
    `Loadtest shared-card baseline ${input.plan.id}`,
    "off_session",
  );
  const baselinePayment = json(baselinePaymentResponse);
  if (baselinePaymentResponse.status < 200 || baselinePaymentResponse.status >= 300 || !baselinePayment.payment_id) {
    throw new Error(`shared card baseline payment creation failed: status=${baselinePaymentResponse.status}`);
  }

  const baselineConfirmResponse = post(`${routerUrl}/payments/${baselinePayment.payment_id}/confirm`, {
    payment_token: token,
    payment_method: "card",
    payment_method_type: "credit",
    setup_future_usage: "off_session",
    customer_acceptance: customerAcceptance(),
  }, apiKeyHeaders(input));
  const baselineConfirm = json(baselineConfirmResponse);
  const savedPaymentMethodId = baselineConfirm.payment_method_id
    || findSavedPaymentMethod(routerUrl, baselinePayment.payment_id);
  if (
    baselineConfirmResponse.status < 200
    || baselineConfirmResponse.status >= 300
    || !savedPaymentMethodId
  ) {
    throw new Error(`shared card baseline confirmation failed: status=${baselineConfirmResponse.status}`);
  }

  return { customer_id: customerId, saved_payment_method_id: savedPaymentMethodId };
}

export function createFixture(shared) {
  const index = exec.scenario.iterationInTest;
  const fixtureId = `fixture_${String(index).padStart(8, "0")}_${input.run_id}`;
  const reference = `customer_loadtest_${input.run_id}_${index}`;
  const routerUrl = input.services.router.replace(/\/$/, "");
  const modularUrl = input.services["modular-pm"]?.replace(/\/$/, "");
  let customerId = shared?.customer_id || null;
  if (input.plan.requiresCustomer && !customerId) {
    const response = createCustomer(modularUrl, reference);
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
    const response = createPmSession(modularUrl, customerId);
    const body = json(response);
    pmSessionId = body.id;
    pmSessionClientSecret = body.client_secret;
    if (response.status < 200 || response.status >= 300 || !pmSessionId || !pmSessionClientSecret) {
      recordFailure(index, "pm_session", response);
      return;
    }
  }
  let savedPaymentMethodId = shared?.saved_payment_method_id || null;
  if (input.plan.requiresSavedCard && !input.plan.sharedCustomer) {
    const baselinePaymentResponse = createPayment(routerUrl, customerId, `Loadtest baseline ${input.plan.id}`);
    const baselinePayment = json(baselinePaymentResponse);
    if (baselinePaymentResponse.status < 200 || baselinePaymentResponse.status >= 300 || !baselinePayment.payment_id) {
      recordFailure(index, "baseline_payment", baselinePaymentResponse);
      return;
    }
    const baselineConfirmResponse = post(`${routerUrl}/payments/${baselinePayment.payment_id}/confirm`, {
      payment_method: "card",
      payment_method_type: "credit",
      payment_method_data: { card: input.card },
      // Fixture setup must finish storing the card before measured traffic starts.
      setup_future_usage: "off_session",
      customer_acceptance: customerAcceptance(),
    }, apiKeyHeaders(input));
    const baselineConfirm = json(baselineConfirmResponse);
    savedPaymentMethodId = baselineConfirm.payment_method_id
      || findSavedPaymentMethod(routerUrl, baselinePayment.payment_id);
    if (
      baselineConfirmResponse.status < 200
      || baselineConfirmResponse.status >= 300
      || !savedPaymentMethodId
    ) {
      recordFailure(index, "baseline_confirm", baselineConfirmResponse);
      return;
    }
  }
  let paymentToken = null;
  if (input.plan.listsSavedPaymentMethods) {
    const response = get(
      `${modularUrl}/payment-method-sessions/${pmSessionId}/list-payment-methods`,
      modularHeaders(input, pmSessionClientSecret),
    );
    const body = json(response);
    const savedCard = body.customer_payment_methods?.find(
      (paymentMethod) => paymentMethod.payment_method_type === "card",
    );
    paymentToken = extractToken(savedCard?.payment_method_token);
    if (response.status < 200 || response.status >= 300 || !paymentToken) {
      recordFailure(index, "list_saved_payment_methods", response);
      return;
    }
  }
  const paymentResponse = createPayment(routerUrl, customerId, `Loadtest automation ${input.plan.id}`);
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
    saved_payment_method_id: savedPaymentMethodId || "",
    payment_token: paymentToken || "",
  });
}
