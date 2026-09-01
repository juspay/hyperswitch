"use strict";

const crypto = require("crypto");
const { spawnSync } = require("child_process");
const fs = require("fs");
const path = require("path");
const { resolveMaybe } = require("../../lib/config");
const { resolveConfiguredValue } = require("./config");
const { latencySummary, phaseLatencySummary } = require("./results");
const { buildPlan } = require("./scenarios");

const DEFAULT_CARD = Object.freeze({
  card_number: "4242424242424242",
  card_exp_month: "10",
  card_exp_year: "35",
  card_holder_name: "Load Test",
  card_cvc: "123",
});
const DEFAULT_METADATA_UPDATE = Object.freeze({
  card_exp_month: "11",
  card_exp_year: "36",
  card_holder_name: "Updated Load Test",
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

function targetHeaders(state, name, headers = {}) {
  return { ...(state.config.target_headers?.[name] || {}), ...headers };
}

function apiKeyHeaders(state, apiKey) {
  return targetHeaders(state, "router", { "api-key": apiKey });
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
  const target = state.config.targets.superposition;
  const baseUrl = target.base_url.replace(/\/$/, "");
  const headers = targetHeaders(state, "superposition", {
    "x-org-id": target.org_id || "localorg",
    "x-workspace": target.workspace_id || "dev",
    ...(target.secret ? { "X-Superposition-Secret": target.secret } : {}),
  });
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
  const waitSeconds = Number(target.propagation_wait_seconds ?? 6);
  if (waitSeconds > 0) await new Promise((resolve) => setTimeout(resolve, waitSeconds * 1000));
}

async function setupMerchant(state, merchantPath, fresh, data) {
  const superpositionMode = state.config.superposition.mode;
  if (state.config.merchant.mode === "existing") {
    const merchant = { ...state.config.resolved_merchant, merchant_path: merchantPath };
    data.merchants[merchant.merchant_id] = merchant;
    data.current_merchants[merchantPath] = merchant.merchant_id;
    if (superpositionMode === "manage") {
      await configureModularRouting(state, merchant, merchantPath === "modular");
    }
    return merchant;
  }
  const currentId = data.current_merchants[merchantPath];
  if (!fresh && currentId && data.merchants[currentId]) return data.merchants[currentId];
  const cfg = apiConfig(state);
  const baseUrl = state.config.services.router.replace(/\/$/, "");
  const adminApiKey = resolveConfiguredValue(cfg, "admin_api_key") || "test_admin";
  const suffix = `${Date.now()}_${crypto.randomUUID().slice(0, 8)}`;
  const requestedMerchantId = `merchant_loadtest_${merchantPath}_${suffix}`;
  const account = (await requestJson("POST", `${baseUrl}/accounts`, {
    merchant_id: requestedMerchantId,
    merchant_name: "Loadtest Automation",
    return_url: "https://example.com/return",
    metadata: { source: "loadtest-automation", merchant_path: merchantPath },
  }, apiKeyHeaders(state, adminApiKey))).body;
  const merchantId = account.merchant_id || requestedMerchantId;
  const apiKey = (await requestJson("POST", `${baseUrl}/api_keys/${merchantId}`, {
    name: `loadtest-${merchantPath}-${suffix}`, description: "Generated for local loadtest automation", expiration: "never",
  }, apiKeyHeaders(state, adminApiKey))).body.api_key;
  const profileLookup = (await requestJson("GET", `${baseUrl}/account/${merchantId}/business_profile`, undefined, apiKeyHeaders(state, apiKey))).body;
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
  }, apiKeyHeaders(state, apiKey))).body;
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
  if (superpositionMode === "manage") {
    await configureModularRouting(state, merchant, merchantPath === "modular");
  }
  return merchant;
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

function remainingFixtureWaitSeconds(run, fixtures) {
  const configuredWait = Number(fixtures?.wait_before_start_seconds || 0);
  if (configuredWait <= 0) return 0;
  const readyAt = Date.parse(run.fixtures_ready_at || run.created_at || "");
  if (!Number.isFinite(readyAt)) return configuredWait;
  return Math.max(0, configuredWait - (Date.now() - readyAt) / 1000);
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
  const temporaryPrefix = path.join(path.dirname(statePath(state)), `.k6-fixtures-${process.pid}-${runId}`);
  const inputPath = `${temporaryPrefix}-input.json`;
  const outputPath = `${temporaryPrefix}-output.json`;
  const input = {
    services: state.config.services,
    target_headers: state.config.target_headers,
    merchant,
    plan,
    run_id: runId,
    count,
    concurrency: Number(state.config.fixtures?.concurrency || 1),
    fixture_config: state.config.fixtures || {},
    card: apiConfig(state).card || DEFAULT_CARD,
    request_timeout_ms: requestTimeoutMs,
  };
  fs.writeFileSync(inputPath, JSON.stringify(input), { mode: 0o600 });
  let execution;
  try {
    execution = spawnSync(process.env.K6_BIN || "k6", [
      "run",
      "--out",
      `json=${outputPath}`,
      path.resolve(__dirname, "../k6/fixtures.js"),
    ], {
      env: { ...process.env, K6_INPUT: inputPath },
      stdio: "inherit",
    });
  } finally {
    fs.rmSync(inputPath, { force: true });
  }
  if (execution.error) throw new Error(`Failed to start k6 fixture preparation: ${execution.error.message}`);
  const created = [];
  const failures = [];
  if (fs.existsSync(outputPath)) {
    for (const line of fs.readFileSync(outputPath, "utf8").split("\n")) {
      if (!line) continue;
      const point = JSON.parse(line);
      if (point.type !== "Point") continue;
      const tags = point.data.tags || {};
      if (point.metric === "loadtest_fixture_created") {
        created.push({
          fixture_id: tags.fixture_id,
          version: 1,
          run_id: runId,
          merchant_id: merchant.merchant_id,
          scenario_id: plan.id,
          payment_id: tags.payment_id,
          customer_id: tags.customer_id || null,
          pm_session_id: tags.pm_session_id || null,
          pm_session_client_secret: tags.pm_session_client_secret || null,
          saved_payment_method_id: tags.saved_payment_method_id || null,
          state: "ready",
        });
      } else if (point.metric === "loadtest_fixture_failed") {
        failures.push(tags.error || "unknown fixture error");
      }
    }
    fs.rmSync(outputPath, { force: true });
  }
  if (execution.status !== 0 || created.length !== count) {
    run.state = "failed";
    writeState(state, data);
    throw new Error(`k6 fixture preparation created ${created.length}/${count}; ${failures[0] || `k6 exited with status ${execution.status}`}`);
  }
  created.sort((left, right) => left.fixture_id.localeCompare(right.fixture_id));
  data.fixtures.push(...created);
  run.state = "ready";
  run.fixtures_ready_at = new Date().toISOString();
  writeState(state, data);
  return { run_id: runId, created: count, merchant_path: plan.merchantPath, scenario: plan.scenarioName };
}

function selectReadyFixtures(data, runId) {
  return data.fixtures.filter((fixture) => fixture.run_id === runId && fixture.state === "ready");
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
  const remainingWaitSeconds = remainingFixtureWaitSeconds(run, state.config.fixtures);
  if (remainingWaitSeconds > 0) {
    process.stderr.write(`Waiting ${Math.ceil(remainingWaitSeconds)}s for fixtures to settle before measured load\n`);
    await new Promise((resolve) => setTimeout(resolve, remainingWaitSeconds * 1000));
  }
  const phases = loadPhases(state.config.load || {});
  const grafanaUrl = state.config.services.grafana_dashboard || state.config.services.grafana || null;
  process.stderr.write(`Grafana: ${grafanaUrl || "not configured"}\n`);
  const temporaryPrefix = path.join(path.dirname(statePath(state)), `.k6-${process.pid}-${run.run_id}`);
  const inputPath = `${temporaryPrefix}-input.json`;
  const outputPath = `${temporaryPrefix}-output.json`;
  const workloadPath = path.resolve(__dirname, "../k6/workload.js");
  const input = {
    services: state.config.services,
    target_headers: state.config.target_headers,
    merchant,
    plan,
    card: apiConfig(state).card || DEFAULT_CARD,
    metadata_update: apiConfig(state).metadata_update || DEFAULT_METADATA_UPDATE,
    request_timeout_ms: requestTimeoutMs,
    phases,
    fixtures,
  };
  fs.writeFileSync(inputPath, JSON.stringify(input), { mode: 0o600 });
  run.state = "running";
  writeState(state, data);
  let execution;
  try {
    execution = spawnSync(process.env.K6_BIN || "k6", [
      "run",
      "--out",
      `json=${outputPath}`,
      workloadPath,
    ], {
      env: { ...process.env, K6_INPUT: inputPath },
      stdio: "inherit",
    });
  } finally {
    fs.rmSync(inputPath, { force: true });
  }
  if (execution.error) throw new Error(`Failed to start k6: ${execution.error.message}`);
  const records = [];
  if (fs.existsSync(outputPath)) {
    for (const line of fs.readFileSync(outputPath, "utf8").split("\n")) {
      if (!line) continue;
      const point = JSON.parse(line);
      if (point.type !== "Point" || point.metric !== "loadtest_result_ms") continue;
      const tags = point.data.tags || {};
      const phase = tags.rps_phase || tags.phase || tags.scenario || null;
      const phaseRpsMatch = String(phase || "").match(/^phase_\d+_(\d+(?:\.\d+)?)_rps$/);
      records.push({
        fixture_id: tags.fixture_id,
        run_id: run.run_id,
        merchant_id: merchant.merchant_id,
        scenario_id: plan.id,
        payment_id: tags.payment_id || null,
        status: tags.result_status,
        error: tags.error || undefined,
        pm_session_confirm_request_id: tags.pm_request_id || null,
        payment_confirm_request_id: tags.payment_request_id || null,
        pm_session_confirm_latency_ms: tags.pm_latency_ms ? Number(tags.pm_latency_ms) : null,
        payment_confirm_latency_ms: Number(tags.payment_latency_ms || 0),
        hyperswitch_internal_latency_ms: tags.hyperswitch_internal_latency_ms
          ? Number(tags.hyperswitch_internal_latency_ms)
          : null,
        combined_internal_latency_ms: tags.combined_internal_latency_ms
          ? Number(tags.combined_internal_latency_ms)
          : null,
        phase,
        phase_rps: tags.phase_rps ? Number(tags.phase_rps) : phaseRpsMatch ? Number(phaseRpsMatch[1]) : null,
        total_latency_ms: Number(point.data.value),
        created_at: point.data.time || new Date().toISOString(),
      });
    }
    fs.rmSync(outputPath, { force: true });
  }
  for (const record of records) {
    const fixture = fixtures.find((candidate) => candidate.fixture_id === record.fixture_id);
    if (fixture) fixture.state = SUCCESS_STATUSES.has(record.status) ? "confirmed" : "failed";
    data.results.push(record);
  }
  run.state = "completed";
  run.completed_at = new Date().toISOString();
  writeState(state, data);
  const results = data.results.filter((result) => result.run_id === run.run_id);
  if (execution.status !== 0) {
    throw new Error(`k6 exited with status ${execution.status}; captured ${records.length} request results`);
  }
  return {
    run_id: run.run_id,
    grafana_url: state.config.services.grafana || null,
    attempted: results.length,
    succeeded: results.filter((result) => SUCCESS_STATUSES.has(result.status)).length,
    failed: results.filter((result) => !SUCCESS_STATUSES.has(result.status)).length,
    fixture_wait_seconds: Math.ceil(remainingWaitSeconds),
    latency: {
      aggregate: latencySummary(results),
      by_phase: phaseLatencySummary(results),
    },
    results,
  };
}

async function smoke(state, args) {
  const merchantPath = args[0] || "non_modular";
  const scenarioName = args[1] || "cit_off_session";
  if (
    state.config.environment_mode === "cloud"
    && (merchantPath !== state.config.scenario.merchant_path || scenarioName !== state.config.scenario.name)
  ) {
    throw new Error("Cloud smoke arguments must match the scenario configured in the runner config");
  }
  state.config.scenario = { merchant_path: merchantPath, name: scenarioName };
  // Smoke confirms routing and a single end-to-end flow; it is not a measured
  // load run, so do not apply the fixture settling delay configured for start.
  state.config.fixtures = {
    ...(state.config.fixtures || {}), count: 1, concurrency: 1, fresh_merchant: true, wait_before_start_seconds: 0,
  };
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
    const probes = {};
    if (state.config.environment_mode === "cloud" && state.config.preflight?.probe_targets !== false) {
      for (const name of state.config.required_targets) {
        const target = state.config.targets[name];
        const response = await requestJson("GET", target.health_url, undefined, targetHeaders(state, name));
        probes[name] = { health_url: target.health_url, status: response.status, request_id: response.requestId };
      }
    }
    const targets = Object.fromEntries(state.config.required_targets.map((name) => [name, {
      base_url: state.config.targets[name].base_url,
      health_url: state.config.targets[name].health_url || null,
      header_names: Object.keys(state.config.target_headers[name] || {}),
    }]));
    return {
      config: state.configPath,
      deployment_config: state.deploymentPath,
      environment_mode: state.config.environment_mode,
      scenario_id: plan.id,
      merchant_mode: state.config.merchant.mode,
      superposition_mode: state.config.superposition.mode,
      targets,
      probes,
      state_file: statePath(state),
    };
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
    const results = run ? data.results.filter((result) => result.run_id === run.run_id) : [];
    return {
      active_run: run,
      fixtures: run ? data.fixtures.filter((fixture) => fixture.run_id === run.run_id).reduce((counts, fixture) => ({ ...counts, [fixture.state]: (counts[fixture.state] || 0) + 1 }), {}) : {},
      latency: run ? { aggregate: latencySummary(results), by_phase: phaseLatencySummary(results) } : null,
      recent_results: results.slice(-10),
    };
  }
  throw new Error(`Unsupported runner action: ${action}`);
}

module.exports = {
  buildPaymentConfirmBody,
  loadPhases,
  plannedFixtureCount,
  remainingFixtureWaitSeconds,
  run,
  selectReadyFixtures,
  setupMerchant,
};
