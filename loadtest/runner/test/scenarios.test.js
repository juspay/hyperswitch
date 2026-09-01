"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const test = require("node:test");
const { parseYaml } = require("../../lib/config");
const {
  requiredTargetNames,
  resolveConfiguredValue,
  resolveRunnerConfiguration,
} = require("../lib/config");
const { latencySummary, percentile, phaseLatencySummary } = require("../lib/results");
const {
  buildPaymentConfirmBody,
  loadPhases,
  remainingFixtureWaitSeconds,
  run,
  selectReadyFixtures,
  setupMerchant,
} = require("../lib/router_api");
const { buildPlan } = require("../lib/scenarios");

const paths = ["non_modular", "modular"];
const scenarios = ["guest", "cit_on_session", "cit_off_session"];

for (const merchantPath of paths) {
  for (const name of scenarios) {
    test(`${merchantPath} ${name} produces a stable plan`, () => {
      const plan = buildPlan({ merchant_path: merchantPath, name });
      assert.equal(plan.id, `${merchantPath}:${name}`);
      assert.equal(plan.usesPmService, merchantPath === "modular");
      assert.equal(plan.requiresCustomer, name !== "guest");
    });
  }
}

test("invalid scenario selections fail before traffic starts", () => {
  assert.throws(() => buildPlan({ merchant_path: "unknown", name: "guest" }), /Unsupported merchant_path/);
  assert.throws(() => buildPlan({ merchant_path: "modular", name: "unknown" }), /Unsupported scenario/);
  assert.throws(
    () => buildPlan({ merchant_path: "modular", name: "cit_metadata_changed" }),
    /only the non_modular merchant path/,
  );
});

test("metadata-change CIT requires a pre-existing saved card", () => {
  const plan = buildPlan({ merchant_path: "non_modular", name: "cit_metadata_changed" });
  assert.equal(plan.requiresSavedCard, true);
  assert.equal(plan.metadataChanged, true);
  assert.equal(plan.setupFutureUsage, "off_session");
});

test("one-time confirms do not contain save-card fields", () => {
  const body = buildPaymentConfirmBody(buildPlan({ merchant_path: "non_modular", name: "guest" }), {}, null);
  assert.equal(body.setup_future_usage, undefined);
  assert.equal(body.customer_acceptance, undefined);
});

test("CIT confirms contain setup future usage and customer acceptance", () => {
  const body = buildPaymentConfirmBody(buildPlan({ merchant_path: "non_modular", name: "cit_off_session" }), {}, null);
  assert.equal(body.setup_future_usage, "off_session");
  assert.equal(body.customer_acceptance.acceptance_type, "online");
});

test("modular CIT session confirmation includes customer acceptance", () => {
  const source = fs.readFileSync(require.resolve("../k6/workload.js"), "utf8");
  assert.match(source, /if \(input\.plan\.setupFutureUsage\) \{\s*pmSessionConfirmBody\.customer_acceptance = customerAcceptance\(\);/);
});

test("load phases ramp once and own their request count", () => {
  assert.deepEqual(loadPhases({ starting_rps: 1, target_rps: 5, step_rps: 2, hold_seconds: 3, idle_seconds: 1 }), [
    { rps: 1, holdSeconds: 3, idleSeconds: 1, requests: 3 },
    { rps: 3, holdSeconds: 3, idleSeconds: 1, requests: 9 },
    { rps: 5, holdSeconds: 3, idleSeconds: 1, requests: 15 },
  ]);
});

test("fixtures cannot leak between runs", () => {
  const data = { fixtures: [{ run_id: "one", state: "ready" }, { run_id: "two", state: "ready" }, { run_id: "one", state: "used" }] };
  assert.deepEqual(selectReadyFixtures(data, "one"), [data.fixtures[0]]);
});

test("fixture settling wait is disabled by default and falls back safely for legacy state", () => {
  assert.equal(remainingFixtureWaitSeconds({ created_at: "not-a-date" }, {}), 0);
  assert.equal(
    remainingFixtureWaitSeconds({ created_at: "not-a-date" }, { wait_before_start_seconds: 60 }),
    60,
  );
});

test("shared YAML parser handles the automation config subset", () => {
  assert.deepEqual(parseYaml("scenario:\n  name: cit_on_session\nload:\n  target_rps: 10\n"), {
    scenario: { name: "cit_on_session" },
    load: { target_rps: 10 },
  });
});

test("local target resolution remains backward compatible with deployment config", () => {
  const config = {
    scenario: { merchant_path: "modular", name: "guest" },
    load: { starting_rps: 1, target_rps: 1, step_rps: 0, hold_seconds: 1, idle_seconds: 0 },
    fixtures: { concurrency: 1 },
  };
  const deployment = {
    application_services: {
      router: { base_url: "http://127.0.0.1:8080" },
      "modular-pm": { base_url: "http://127.0.0.1:8081" },
      superposition: { base_url: "http://127.0.0.1:8082", org_id: "localorg", workspace_id: "dev" },
    },
  };
  const resolved = resolveRunnerConfiguration(config, deployment, {});
  assert.equal(resolved.environment_mode, "local");
  assert.equal(resolved.services.router, "http://127.0.0.1:8080");
  assert.equal(resolved.services["modular-pm"], "http://127.0.0.1:8081/v2");
  assert.equal(resolved.superposition.mode, "manage");
});

test("cloud targets support independent headers and do not require deployment config", () => {
  const config = {
    environment: { mode: "cloud" },
    scenario: { merchant_path: "modular", name: "guest" },
    load: { starting_rps: 1, target_rps: 1, step_rps: 0, hold_seconds: 1, idle_seconds: 0 },
    fixtures: { concurrency: 1 },
    targets: {
      router: {
        base_url: "https://sandbox.example.com",
        health_url: "/health",
        headers: { "x-feature": "router-loadtest" },
      },
      modular_pm: {
        base_url: "https://sandbox.example.com",
        api_prefix: "/v1",
        health_url: "/v2/health",
        headers: { "x-feature": "pm-loadtest" },
      },
    },
    merchant: {
      mode: "existing",
      merchant_id: "merchant_123",
      api_key_env: "TEST_MERCHANT_API_KEY",
      publishable_key: "pk_123",
      profile_id: "profile_123",
    },
    superposition: { mode: "preconfigured" },
  };
  const resolved = resolveRunnerConfiguration(config, {}, { TEST_MERCHANT_API_KEY: "secret-key" });
  assert.equal(resolved.environment_mode, "cloud");
  assert.equal(resolved.services["modular-pm"], "https://sandbox.example.com/v1");
  assert.deepEqual(resolved.target_headers.router, { "x-feature": "router-loadtest" });
  assert.deepEqual(resolved.target_headers["modular-pm"], { "x-feature": "pm-loadtest" });
  assert.equal(resolved.resolved_merchant.merchant_api_key, "secret-key");
  assert.equal(resolved.targets.router.health_url, "https://sandbox.example.com/health");
});

test("configured secrets can be sourced from the environment", () => {
  assert.equal(resolveConfiguredValue({ api_key_env: "LOADTEST_KEY" }, "api_key", { LOADTEST_KEY: "secret" }), "secret");
  assert.throws(
    () => resolveConfiguredValue({ api_key_env: "LOADTEST_KEY" }, "api_key", {}),
    /LOADTEST_KEY is not set/,
  );
});

test("required targets follow fixture and measured flow requirements", () => {
  assert.deepEqual(requiredTargetNames(buildPlan({ merchant_path: "non_modular", name: "guest" }), "disabled"), ["router"]);
  assert.deepEqual(
    requiredTargetNames(buildPlan({ merchant_path: "non_modular", name: "cit_off_session" }), "preconfigured"),
    ["router", "modular-pm"],
  );
  assert.deepEqual(
    requiredTargetNames(buildPlan({ merchant_path: "modular", name: "guest" }), "manage"),
    ["router", "modular-pm", "superposition"],
  );
});

test("cloud configuration rejects missing required targets", () => {
  assert.throws(
    () => resolveRunnerConfiguration({
      environment: { mode: "cloud" },
      scenario: { merchant_path: "modular", name: "guest" },
      load: { starting_rps: 1, target_rps: 1, step_rps: 0, hold_seconds: 1, idle_seconds: 0 },
      fixtures: { concurrency: 1 },
      targets: { router: { base_url: "https://sandbox.example.com", health_url: "/health" } },
      merchant: {
        mode: "existing", merchant_id: "merchant_123", api_key: "secret", publishable_key: "pk_123", profile_id: "profile_123",
      },
      superposition: { mode: "preconfigured" },
    }, {}, {}),
    /modular-pm target base_url is required/,
  );
});

test("non-modular customer fixtures do not require a publishable key", () => {
  const resolved = resolveRunnerConfiguration({
    environment: { mode: "cloud" },
    preflight: { probe_targets: false },
    scenario: { merchant_path: "non_modular", name: "cit_off_session" },
    load: { starting_rps: 1, target_rps: 1, step_rps: 0, hold_seconds: 1, idle_seconds: 0 },
    fixtures: { concurrency: 1 },
    targets: {
      router: { base_url: "https://router.example.com" },
      modular_pm: { base_url: "https://pm.example.com" },
    },
    merchant: { mode: "existing", merchant_id: "merchant_123", api_key: "secret", profile_id: "profile_123" },
    superposition: { mode: "preconfigured" },
  }, {}, {});
  assert.equal(resolved.resolved_merchant.publishable_key, undefined);
  assert.deepEqual(resolved.required_targets, ["router", "modular-pm"]);
});

test("latency summaries include stable percentile rows", () => {
  assert.equal(percentile([10, 20, 30, 40], 50), 25);
  assert.deepEqual(latencySummary([
    { payment_confirm_latency_ms: 10, hyperswitch_internal_latency_ms: 7, combined_internal_latency_ms: 12, total_latency_ms: 15, pm_session_confirm_latency_ms: 5 },
    { payment_confirm_latency_ms: 20, hyperswitch_internal_latency_ms: 12, combined_internal_latency_ms: 22, total_latency_ms: 30, pm_session_confirm_latency_ms: 10 },
  ]), [
    { metric: "pm_session_confirm", count: 2, avg_ms: 7.5, p50_ms: 7.5, p75_ms: 8.75, p90_ms: 9.5, p99_ms: 9.95, max_ms: 10 },
    { metric: "payment_confirm", count: 2, avg_ms: 15, p50_ms: 15, p75_ms: 17.5, p90_ms: 19, p99_ms: 19.9, max_ms: 20 },
    { metric: "hyperswitch_internal_excluding_connector", count: 2, avg_ms: 9.5, p50_ms: 9.5, p75_ms: 10.75, p90_ms: 11.5, p99_ms: 11.95, max_ms: 12 },
    { metric: "combined", count: 2, avg_ms: 17, p50_ms: 17, p75_ms: 19.5, p90_ms: 21, p99_ms: 21.9, max_ms: 22 },
  ]);
});

test("phase latency summaries retain each phase's time bounds and aggregate", () => {
  const summary = phaseLatencySummary([
    { phase: "phase_1_10_rps", phase_rps: 10, payment_confirm_latency_ms: 10, combined_internal_latency_ms: 10, total_latency_ms: 10, created_at: "2026-08-11T10:00:00.000Z" },
    { phase: "phase_1_10_rps", phase_rps: 10, payment_confirm_latency_ms: 20, combined_internal_latency_ms: 20, total_latency_ms: 20, created_at: "2026-08-11T10:00:05.000Z" },
    { phase: "phase_2_20_rps", phase_rps: 20, payment_confirm_latency_ms: 30, combined_internal_latency_ms: 30, total_latency_ms: 30, created_at: "2026-08-11T10:00:10.000Z" },
  ]);
  assert.equal(summary.length, 2);
  assert.deepEqual(summary[0], {
    phase: "phase_1_10_rps",
    rps: 10,
    started_at: "2026-08-11T10:00:00.000Z",
    ended_at: "2026-08-11T10:00:05.000Z",
    latency: [
      { metric: "payment_confirm", count: 2, avg_ms: 15, p50_ms: 15, p75_ms: 17.5, p90_ms: 19, p99_ms: 19.9, max_ms: 20 },
      { metric: "combined", count: 2, avg_ms: 15, p50_ms: 15, p75_ms: 17.5, p90_ms: 19, p99_ms: 19.9, max_ms: 20 },
    ],
  });
});

test("k6 request headers merge independent target headers with authentication", async () => {
  const source = fs.readFileSync(require.resolve("../k6/headers.js"), "utf8");
  const moduleUrl = `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`;
  const headers = await import(moduleUrl);
  const input = {
    target_headers: {
      router: { "x-feature": "router-loadtest", "x-hs-latency": "true" },
      "modular-pm": { "x-feature": "pm-loadtest" },
    },
    merchant: { merchant_api_key: "merchant-key", publishable_key: "publishable-key", profile_id: "profile-id" },
  };
  assert.deepEqual(headers.apiKeyHeaders(input), {
    "x-feature": "router-loadtest", "x-hs-latency": "true", "api-key": "merchant-key", "content-type": "application/json",
  });
  assert.deepEqual(headers.modularHeaders(input, "client-secret"), {
    "x-feature": "pm-loadtest",
    Authorization: "publishable-key=publishable-key,client-secret=client-secret",
    "x-profile-id": "profile-id",
    "content-type": "application/json",
  });
});

test("existing merchant mode avoids administrative requests", async () => {
  const merchant = {
    merchant_id: "merchant_123",
    merchant_api_key: "merchant-key",
    publishable_key: "publishable-key",
    profile_id: "profile-id",
    organization_id: "organization-id",
  };
  const state = {
    config: {
      merchant: { mode: "existing" },
      resolved_merchant: merchant,
      superposition: { mode: "preconfigured" },
    },
  };
  const data = { merchants: {}, current_merchants: {} };
  const originalFetch = global.fetch;
  global.fetch = async () => { throw new Error("existing merchant mode must not call fetch"); };
  try {
    assert.deepEqual(await setupMerchant(state, "modular", true, data), { ...merchant, merchant_path: "modular" });
    assert.equal(data.current_merchants.modular, "merchant_123");
  } finally {
    global.fetch = originalFetch;
  }
});

test("cloud merchant provisioning propagates router target headers", async () => {
  const responses = [
    { merchant_id: "merchant_123", publishable_key: "publishable-key", organization_id: "organization-id" },
    { api_key: "merchant-key" },
    [{ profile_id: "profile-id", profile_name: "default" }],
    { merchant_connector_id: "connector-id" },
  ];
  const calls = [];
  const originalFetch = global.fetch;
  global.fetch = async (url, options) => {
    calls.push({ url, options });
    return new Response(JSON.stringify(responses.shift()), {
      status: 200,
      headers: { "content-type": "application/json" },
    });
  };
  const state = {
    config: {
      merchant: { mode: "create" },
      superposition: { mode: "disabled" },
      services: { router: "https://sandbox.example.com" },
      target_headers: { router: { "x-feature": "router-loadtest" } },
      payments_api: { admin_api_key: "admin-key", connector_name: "stripe_test" },
    },
  };
  const data = { merchants: {}, current_merchants: {} };
  try {
    const merchant = await setupMerchant(state, "non_modular", true, data);
    assert.equal(merchant.merchant_id, "merchant_123");
    assert.equal(calls.length, 4);
    assert.ok(calls.every((call) => call.options.headers["x-feature"] === "router-loadtest"));
  } finally {
    global.fetch = originalFetch;
  }
});

test("cloud preflight probes required targets with their own headers", async () => {
  const calls = [];
  const originalFetch = global.fetch;
  global.fetch = async (url, options) => {
    calls.push({ url, options });
    return new Response("{}", { status: 200, headers: { "x-request-id": `${calls.length}` } });
  };
  const state = {
    configPath: "/tmp/cloud.yaml",
    deploymentPath: null,
    configDir: "/tmp",
    config: {
      environment_mode: "cloud",
      scenario: { merchant_path: "modular", name: "guest" },
      merchant: { mode: "existing" },
      superposition: { mode: "preconfigured" },
      preflight: { probe_targets: true },
      required_targets: ["router", "modular-pm"],
      targets: {
        router: { base_url: "https://sandbox.example.com", health_url: "https://sandbox.example.com/health" },
        "modular-pm": { base_url: "https://sandbox.example.com/v2", health_url: "https://sandbox.example.com/v2/health" },
      },
      target_headers: {
        router: { "x-feature": "router-loadtest" },
        "modular-pm": { "x-feature": "pm-loadtest" },
      },
      payments_api: { request_timeout_ms: 1000 },
      state_file: "state/cloud.json",
    },
  };
  try {
    const result = await run("preflight", state);
    assert.equal(result.environment_mode, "cloud");
    assert.deepEqual(result.targets.router.header_names, ["x-feature"]);
    assert.equal(calls[0].options.headers["x-feature"], "router-loadtest");
    assert.equal(calls[1].options.headers["x-feature"], "pm-loadtest");
  } finally {
    global.fetch = originalFetch;
  }
});
