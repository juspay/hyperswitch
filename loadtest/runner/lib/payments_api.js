"use strict";

const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const { resolveMaybe } = require("../../lib/config");
const { buildPlan } = require("./scenarios");

const DEFAULT_CARD = Object.freeze({
  card_number: "4242424242424242",
  card_exp_month: "10",
  card_exp_year: "35",
  card_holder_name: "Load Test",
  card_cvc: "123",
});
const SUCCESS_STATUSES = new Set(["succeeded", "requires_capture", "processing"]);
let requestTimeoutMs = 30000;

function apiConfig(state) {
  return state.config.payments_api || {};
}

function statePath(state) {
  return resolveMaybe(state.configDir, state.config.state_file || "state/loadtest.json");
}

function emptyState() {
  return { version: 2, merchants: {}, current_merchants: {}, runs: {}, active_run_id: null, fixtures: [], results: [] };
}

function readState(state) {
  const file = statePath(state);
  if (!fs.existsSync(file)) return emptyState();
  const data = JSON.parse(fs.readFileSync(file, "utf8"));
  return data.version === 2 ? data : emptyState();
}

function writeState(state, data) {
  const file = statePath(state);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, `${JSON.stringify(data, null, 2)}\n`);
}

async function requestJson(method, url, body, headers = {}, allowedStatuses = []) {
  let response;
  try {
    response = await fetch(url, {
      method,
      headers: { accept: "application/json", ...(body === undefined ? {} : { "content-type": "application/json" }), ...headers },
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(requestTimeoutMs),
    });
  } catch (error) {
    throw new Error(`${method} ${url} failed: ${error.message}`);
  }
  const text = await response.text();
  let payload;
  try { payload = text ? JSON.parse(text) : {}; } catch { payload = { raw: text }; }
  if (!response.ok && !allowedStatuses.includes(response.status)) {
    throw new Error(`${method} ${url} failed: status=${response.status} body=${JSON.stringify(payload).slice(0, 800)}`);
  }
  return { body: payload, requestId: response.headers.get("x-request-id"), status: response.status };
}

function apiKeyHeaders(apiKey) {
  return { "api-key": apiKey };
}

function modularHeaders(merchant, clientSecret) {
  return {
    Authorization: clientSecret ? `publishable-key=${merchant.publishable_key},client-secret=${clientSecret}` : `api-key=${merchant.merchant_api_key}`,
    "x-profile-id": merchant.profile_id,
    "x-feature": "sandbox-pm-loadtest",
  };
}

function customerAcceptance() {
  return {
    acceptance_type: "online",
    accepted_at: new Date().toISOString(),
    online: { ip_address: "127.0.0.1", user_agent: "loadtest-automation-runner" },
  };
}

async function ensureSuperpositionResource(baseUrl, resource, name, body, headers) {
  const existing = await requestJson("GET", `${baseUrl}/${resource}/${name}`, undefined, headers, [404]);
  if (existing.status === 404) await requestJson("POST", `${baseUrl}/${resource}`, body, headers);
}

async function configureModularRouting(state, merchant, enabled) {
  if (!merchant.organization_id) throw new Error("merchant setup did not return organization_id");
  const services = state.config.services;
  const baseUrl = services.superposition.replace(/\/$/, "");
  const headers = {
    "x-org-id": services.superposition_org_id || "localorg",
    "x-workspace": services.superposition_workspace_id || "dev",
    ...(services.superposition_secret ? { "X-Superposition-Secret": services.superposition_secret } : {}),
  };
  await ensureSuperpositionResource(baseUrl, "dimension", "organization_id", {
    dimension: "organization_id", position: 1, schema: { type: "string" }, value_validation_function_name: null,
    description: "Hyperswitch organization identifier", change_reason: "Initialize local loadtest workspace", value_compute_function_name: null,
  }, headers);
  await ensureSuperpositionResource(baseUrl, "default-config", "should_call_pm_modular_service", {
    key: "should_call_pm_modular_service", value: false, schema: { type: "boolean" }, value_validation_function_name: null,
    description: "Route payment-method operations through PM modular", change_reason: "Initialize local loadtest workspace", value_compute_function_name: null,
  }, headers);
  await requestJson("PUT", `${baseUrl}/context`, {
    override: { should_call_pm_modular_service: enabled }, context: { organization_id: merchant.organization_id },
    description: `Loadtest automation: ${enabled ? "modular" : "non-modular"} payments`, change_reason: "local loadtest automation",
  }, headers);
  const waitSeconds = Number(services.superposition_propagation_wait_seconds ?? 6);
  if (waitSeconds > 0) await new Promise((resolve) => setTimeout(resolve, waitSeconds * 1000));
}

async function setupMerchant(state, merchantPath, fresh, data) {
  const currentId = data.current_merchants[merchantPath];
  if (!fresh && currentId && data.merchants[currentId]) return data.merchants[currentId];
  const cfg = apiConfig(state);
  const baseUrl = state.config.services.payments.replace(/\/$/, "");
  const adminApiKey = cfg.admin_api_key || "test_admin";
  const suffix = `${Date.now()}_${crypto.randomUUID().slice(0, 8)}`;
  const requestedMerchantId = `merchant_loadtest_${merchantPath}_${suffix}`;
  const account = (await requestJson("POST", `${baseUrl}/accounts`, {
    merchant_id: requestedMerchantId,
    merchant_name: "Loadtest Automation",
    return_url: "https://example.com/return",
    metadata: { source: "loadtest-automation", merchant_path: merchantPath },
  }, apiKeyHeaders(adminApiKey))).body;
  const merchantId = account.merchant_id || requestedMerchantId;
  const apiKey = (await requestJson("POST", `${baseUrl}/api_keys/${merchantId}`, {
    name: `loadtest-${merchantPath}-${suffix}`, description: "Generated for local loadtest automation", expiration: "never",
  }, apiKeyHeaders(adminApiKey))).body.api_key;
  const profileLookup = (await requestJson("GET", `${baseUrl}/account/${merchantId}/business_profile`, undefined, apiKeyHeaders(apiKey))).body;
  const profiles = Array.isArray(profileLookup) ? profileLookup : [profileLookup];
  const profile = profiles.find((item) => item.profile_name === "default") || profiles[0];
  if (!profile?.profile_id) throw new Error(`default profile lookup failed: ${JSON.stringify(profileLookup)}`);
  const connector = (await requestJson("POST", `${baseUrl}/account/${merchantId}/connectors`, {
    connector_type: "payment_processor",
    connector_name: cfg.connector_name || "stripe_test",
    connector_label: "loadtest_automation",
    profile_id: profile.profile_id,
    connector_account_details: { auth_type: "HeaderKey", api_key: "dummy_api_key" },
    test_mode: true,
    disabled: false,
    payment_methods_enabled: [{
      payment_method: "card",
      payment_method_types: [{
        payment_method_type: "credit", card_networks: ["Visa", "Mastercard"], minimum_amount: 1,
        maximum_amount: 99999999, recurring_enabled: true, installment_payment_enabled: false,
      }],
    }],
  }, apiKeyHeaders(apiKey))).body;
  const merchant = {
    merchant_id: merchantId,
    merchant_path: merchantPath,
    merchant_api_key: apiKey,
    publishable_key: account.publishable_key,
    organization_id: account.organization_id,
    profile_id: profile.profile_id,
    merchant_connector_id: connector.merchant_connector_id,
  };
  data.merchants[merchantId] = merchant;
  data.current_merchants[merchantPath] = merchantId;
  await configureModularRouting(state, merchant, merchantPath === "modular");
  return merchant;
}

async function createCustomer(state, merchant, plan, reference) {
  if (!plan.requiresCustomer) return null;
  if (plan.usesPmService) {
    const baseUrl = state.config.services.modular.replace(/\/$/, "");
    const body = (await requestJson("POST", `${baseUrl}/v2/customers`, {
      merchant_reference_id: reference, name: "Loadtest Modular User", phone: "6168205362",
      email: `${reference}@example.com`, phone_country_code: "+1",
    }, modularHeaders(merchant))).body;
    return body.id || body.customer_id;
  }
  const baseUrl = state.config.services.payments.replace(/\/$/, "");
  const body = (await requestJson("POST", `${baseUrl}/customers`, {
    customer_id: reference, name: "Loadtest User", phone: "6168205362",
    email: `${reference}@example.com`, phone_country_code: "+1",
  }, apiKeyHeaders(merchant.merchant_api_key))).body;
  return body.customer_id || body.id;
}

async function createFixture(state, data, run, merchant, plan, index) {
  const fixtures = state.config.fixtures || {};
  const baseUrl = state.config.services.payments.replace(/\/$/, "");
  const modularUrl = state.config.services.modular.replace(/\/$/, "");
  const reference = `customer_loadtest_${run.run_id}_${index}`;
  const customerId = await createCustomer(state, merchant, plan, reference);
  if (plan.requiresCustomer && !customerId) throw new Error("customer response has no id");
  let pmSessionId = null;
  let pmSessionClientSecret = null;
  if (plan.usesPmService) {
    const session = (await requestJson("POST", `${modularUrl}/v2/payment-method-sessions`, {
      ...(customerId ? { customer_id: customerId } : {}),
      expires_in: Number(fixtures.session_expiry || 900),
      storage_type: plan.storageType,
    }, modularHeaders(merchant))).body;
    pmSessionId = session.id;
    pmSessionClientSecret = session.client_secret;
    if (!pmSessionId || !pmSessionClientSecret) throw new Error(`PMS session response is incomplete: ${JSON.stringify(session)}`);
  }
  const payment = (await requestJson("POST", `${baseUrl}/payments`, {
    amount: Number(fixtures.amount || 1000),
    currency: fixtures.currency || "USD",
    confirm: false,
    capture_method: "automatic",
    profile_id: merchant.profile_id,
    session_expiry: Number(fixtures.session_expiry || 900),
    description: `Loadtest automation ${plan.id}`,
    ...(customerId ? { customer_id: customerId } : {}),
    ...(plan.setupFutureUsage ? { setup_future_usage: plan.setupFutureUsage } : {}),
  }, apiKeyHeaders(merchant.merchant_api_key))).body;
  if (!payment.payment_id) throw new Error(`payment fixture response is incomplete: ${JSON.stringify(payment)}`);
  return {
    fixture_id: crypto.randomUUID(),
    version: 1,
    run_id: run.run_id,
    merchant_id: merchant.merchant_id,
    scenario_id: plan.id,
    payment_id: payment.payment_id,
    customer_id: customerId,
    pm_session_id: pmSessionId,
    pm_session_client_secret: pmSessionClientSecret,
    state: "ready",
  };
}

function buildPaymentConfirmBody(plan, card, token) {
  const body = plan.usesPmService
    ? { payment_token: token, payment_method: "card", payment_method_type: "credit" }
    : { payment_method: "card", payment_method_type: "credit", payment_method_data: { card } };
  if (plan.setupFutureUsage) {
    body.setup_future_usage = plan.setupFutureUsage;
    body.customer_acceptance = customerAcceptance();
  }
  return body;
}

async function confirmFixture(state, merchant, fixture, plan) {
  const card = apiConfig(state).card || DEFAULT_CARD;
  let token = null;
  let pmRequestId = null;
  let pmLatencyMs = null;
  if (plan.usesPmService) {
    const started = performance.now();
    const response = await requestJson("POST", `${state.config.services.modular.replace(/\/$/, "")}/v2/payment-method-sessions/${fixture.pm_session_id}/confirm`, {
      payment_method_data: { card }, payment_method_type: "card", payment_method_subtype: "credit",
    }, modularHeaders(merchant, fixture.pm_session_client_secret));
    pmLatencyMs = performance.now() - started;
    pmRequestId = response.requestId;
    token = response.body.associated_payment_methods?.[0]?.payment_method_token;
    if (token && typeof token === "object") token = token.data;
    if (!token) throw new Error(`PMS confirm response has no payment token: ${JSON.stringify(response.body)}`);
  }
  const started = performance.now();
  const payment = await requestJson("POST", `${state.config.services.payments.replace(/\/$/, "")}/payments/${fixture.payment_id}/confirm`,
    buildPaymentConfirmBody(plan, card, token), apiKeyHeaders(merchant.merchant_api_key));
  const paymentLatencyMs = performance.now() - started;
  return {
    fixture_id: fixture.fixture_id,
    run_id: fixture.run_id,
    merchant_id: fixture.merchant_id,
    scenario_id: fixture.scenario_id,
    payment_id: payment.body.payment_id,
    status: payment.body.status,
    pm_session_confirm_request_id: pmRequestId,
    payment_confirm_request_id: payment.requestId,
    pm_session_confirm_latency_ms: pmLatencyMs == null ? null : Number(pmLatencyMs.toFixed(2)),
    payment_confirm_latency_ms: Number(paymentLatencyMs.toFixed(2)),
    total_latency_ms: Number(((pmLatencyMs || 0) + paymentLatencyMs).toFixed(2)),
  };
}

function loadPhases(config) {
  const start = Number(config.starting_rps || 1);
  const target = Number(config.target_rps || start);
  const step = Number(config.step_rps ?? target);
  const holdSeconds = Number(config.hold_seconds || 1);
  const idleSeconds = Number(config.idle_seconds || 0);
  const phases = [];
  for (let rps = start; ; rps = Math.min(target, rps + step)) {
    phases.push({ rps, holdSeconds, idleSeconds, requests: Math.ceil(rps * holdSeconds) });
    if (rps >= target) break;
  }
  return phases;
}

function plannedFixtureCount(state) {
  const configured = state.config.fixtures?.count;
  if (configured !== undefined && configured !== null && configured !== "auto") return Math.max(0, Number(configured));
  return loadPhases(state.config.load || {}).reduce((sum, phase) => sum + phase.requests, 0);
}

async function mapLimit(count, limit, worker) {
  let next = 0;
  const workers = Array.from({ length: Math.min(Math.max(1, limit), count) }, async () => {
    while (next < count) {
      const index = next;
      next += 1;
      await worker(index);
    }
  });
  await Promise.all(workers);
}

async function prepareFixtures(state) {
  const data = readState(state);
  const plan = buildPlan(state.config.scenario || {});
  const merchant = await setupMerchant(state, plan.merchantPath, Boolean(state.config.fixtures?.fresh_merchant), data);
  const runId = `${Date.now()}-${crypto.randomUUID().slice(0, 8)}`;
  const run = {
    run_id: runId,
    scenario_id: plan.id,
    merchant_id: merchant.merchant_id,
    created_at: new Date().toISOString(),
    state: "preparing",
  };
  data.runs[runId] = run;
  data.active_run_id = runId;
  writeState(state, data);
  const count = plannedFixtureCount(state);
  const created = new Array(count);
  await mapLimit(count, Number(state.config.fixtures?.concurrency || 1), async (index) => {
    created[index] = await createFixture(state, data, run, merchant, plan, index);
  });
  data.fixtures.push(...created);
  run.state = "ready";
  writeState(state, data);
  return { run_id: runId, created: count, merchant_path: plan.merchantPath, scenario: plan.scenarioName };
}

function selectReadyFixtures(data, runId) {
  return data.fixtures.filter((fixture) => fixture.run_id === runId && fixture.state === "ready");
}

function delay(milliseconds) {
  return milliseconds > 0 ? new Promise((resolve) => setTimeout(resolve, milliseconds)) : Promise.resolve();
}

async function startRun(state) {
  const data = readState(state);
  const run = data.runs[data.active_run_id];
  if (!run) throw new Error("No prepared active run; run runner-fixtures first");
  const plan = buildPlan(state.config.scenario || {});
  if (run.scenario_id !== plan.id) throw new Error(`Active run is ${run.scenario_id}, requested ${plan.id}`);
  const merchant = data.merchants[run.merchant_id];
  if (!merchant) throw new Error(`Merchant missing for active run: ${run.merchant_id}`);
  const fixtures = selectReadyFixtures(data, run.run_id);
  if (!fixtures.length) throw new Error("No ready fixtures for active run");
  run.state = "running";
  writeState(state, data);
  const inFlight = new Set();
  let fixtureIndex = 0;
  const concurrency = Number(state.config.fixtures?.concurrency || 1);
  const execute = (fixture) => {
    const task = confirmFixture(state, merchant, fixture, plan)
      .catch((error) => ({ fixture_id: fixture.fixture_id, run_id: run.run_id, merchant_id: merchant.merchant_id, scenario_id: plan.id, status: "failed", error: error.message }))
      .then((result) => {
        fixture.state = SUCCESS_STATUSES.has(result.status) ? "confirmed" : "failed";
        const stored = { ...result, created_at: new Date().toISOString() };
        data.results.push(stored);
        writeState(state, data);
        return stored;
      })
      .finally(() => inFlight.delete(task));
    inFlight.add(task);
  };
  for (const phase of loadPhases(state.config.load || {})) {
    const phaseStarted = performance.now();
    for (let index = 0; index < phase.requests && fixtureIndex < fixtures.length; index += 1) {
      await delay(phaseStarted + (index * 1000) / phase.rps - performance.now());
      while (inFlight.size >= concurrency) await Promise.race(inFlight);
      execute(fixtures[fixtureIndex]);
      fixtureIndex += 1;
    }
    await Promise.all(inFlight);
    if (fixtureIndex >= fixtures.length) break;
    await delay(phase.idleSeconds * 1000);
  }
  await Promise.all(inFlight);
  run.state = "completed";
  run.completed_at = new Date().toISOString();
  writeState(state, data);
  const results = data.results.filter((result) => result.run_id === run.run_id);
  return {
    run_id: run.run_id,
    attempted: results.length,
    succeeded: results.filter((result) => SUCCESS_STATUSES.has(result.status)).length,
    failed: results.filter((result) => !SUCCESS_STATUSES.has(result.status)).length,
    results,
  };
}

async function smoke(state, args) {
  const merchantPath = args[0] || "non_modular";
  const scenarioName = args[1] || "cit_off_session";
  state.config.scenario = { merchant_path: merchantPath, name: scenarioName };
  state.config.fixtures = { ...(state.config.fixtures || {}), count: 1, concurrency: 1, fresh_merchant: true };
  state.config.load = { starting_rps: 1, target_rps: 1, step_rps: 0, hold_seconds: 1, idle_seconds: 0 };
  const prepared = await prepareFixtures(state);
  const completed = await startRun(state);
  if (completed.succeeded !== 1) throw new Error(`Smoke failed: ${JSON.stringify(completed)}`);
  return { ...prepared, ...completed };
}

async function run(action, state, args = []) {
  requestTimeoutMs = Number(apiConfig(state).request_timeout_ms || 30000);
  if (action === "preflight") {
    const plan = buildPlan(state.config.scenario || {});
    return { config: state.configPath, deployment_config: state.deploymentPath, scenario_id: plan.id, services: state.config.services, state_file: statePath(state) };
  }
  if (action === "fixtures") return prepareFixtures(state);
  if (action === "start") return startRun(state);
  if (action === "smoke") return smoke(state, args);
  if (action === "discard-fixtures") {
    const data = readState(state);
    const runId = data.active_run_id;
    let discarded = 0;
    for (const fixture of data.fixtures) if (fixture.run_id === runId && fixture.state === "ready") { fixture.state = "discarded"; discarded += 1; }
    writeState(state, data);
    return { run_id: runId, discarded };
  }
  if (action === "status") {
    const data = readState(state);
    const run = data.runs[data.active_run_id] || null;
    return {
      active_run: run,
      fixtures: run ? data.fixtures.filter((fixture) => fixture.run_id === run.run_id).reduce((counts, fixture) => ({ ...counts, [fixture.state]: (counts[fixture.state] || 0) + 1 }), {}) : {},
      recent_results: run ? data.results.filter((result) => result.run_id === run.run_id).slice(-10) : [],
    };
  }
  throw new Error(`Unsupported runner action: ${action}`);
}

module.exports = { buildPaymentConfirmBody, loadPhases, plannedFixtureCount, run, selectReadyFixtures };
