#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");
const { loadYaml, resolveMaybe } = require("../../lib/config");
const { buildPlan } = require("./scenarios");

const LOADTEST_ROOT = path.resolve(__dirname, "../..");
const CONFIG_PATH = path.resolve(LOADTEST_ROOT, process.env.CONFIG || "runner/config.yaml");
const EXAMPLE_CONFIG_PATH = path.resolve(LOADTEST_ROOT, "runner/config.example.yaml");

function loadConfig() {
  const configPath = fs.existsSync(CONFIG_PATH) ? CONFIG_PATH : EXAMPLE_CONFIG_PATH;
  const configDir = path.dirname(configPath);
  const config = loadYaml(configPath);
  const deploymentPath = resolveMaybe(configDir, config.deployment_config || "../deploy/config.yaml");
  const deployment = loadYaml(deploymentPath);
  const applications = deployment.application_services || {};
  const superposition = applications.superposition || {};
  const observability = deployment.observability || {};
  const grafanaBaseUrl = observability.grafana_url?.replace(/\/$/, "");
  const grafanaDashboardUrl = grafanaBaseUrl && observability.grafana_dashboard_uid
    ? `${grafanaBaseUrl}/d/${observability.grafana_dashboard_uid}/${observability.grafana_dashboard_slug || "loadtest"}?orgId=1&from=now-15m&to=now&refresh=5s`
    : grafanaBaseUrl;
  config.services = {
    router: applications.router?.base_url,
    "modular-pm": applications["modular-pm"]?.base_url,
    superposition: superposition.base_url,
    superposition_org_id: superposition.org_id,
    superposition_workspace_id: superposition.workspace_id,
    superposition_secret: superposition.secret,
    superposition_propagation_wait_seconds: superposition.propagation_wait_seconds,
    grafana: grafanaBaseUrl,
    grafana_dashboard: grafanaDashboardUrl,
  };
  validateConfig(config);
  return { config, deployment, deploymentPath, configPath, configDir };
}

function positiveNumber(value, name, allowZero = false) {
  const number = Number(value);
  if (!Number.isFinite(number) || (allowZero ? number < 0 : number <= 0)) throw new Error(`${name} must be ${allowZero ? "non-negative" : "positive"}`);
  return number;
}

function validateConfig(config) {
  const plan = buildPlan(config.scenario || {});
  if (!config.services.router) throw new Error("deployment application_services.router.base_url is required");
  if (!config.services["modular-pm"]) throw new Error("deployment application_services.modular-pm.base_url is required");
  if (plan.usesPmService && !config.services.superposition) throw new Error("deployment application_services.superposition.base_url is required for modular scenarios");
  const load = config.load || {};
  const start = positiveNumber(load.starting_rps || 1, "load.starting_rps");
  const target = positiveNumber(load.target_rps || start, "load.target_rps");
  const step = positiveNumber(load.step_rps ?? target, "load.step_rps", true);
  positiveNumber(load.hold_seconds || 1, "load.hold_seconds");
  positiveNumber(load.idle_seconds || 0, "load.idle_seconds", true);
  if (start > target) throw new Error("load.starting_rps cannot exceed load.target_rps");
  if (start < target && step === 0) throw new Error("load.step_rps must be positive when ramping");
  positiveNumber((config.fixtures || {}).concurrency || 1, "fixtures.concurrency");
}

function nearestRank(values, percentile) {
  const sorted = values.filter(Number.isFinite).sort((left, right) => left - right);
  if (!sorted.length) return null;
  return sorted[Math.max(0, Math.ceil((percentile / 100) * sorted.length) - 1)];
}

function latencySummary(results) {
  const succeeded = results.filter((result) => result.status === "succeeded");
  const definitions = [
    ["PM session confirm", "pm_session_confirm_latency_ms"],
    ["Payment confirm", "payment_confirm_latency_ms"],
    ["Combined", "total_latency_ms"],
  ];
  const hasPmConfirm = succeeded.some((result) => Number(result.pm_session_confirm_latency_ms) > 0);
  return definitions
    .filter(([label]) => hasPmConfirm || label === "Payment confirm")
    .map(([metric, key]) => {
      const values = succeeded.map((result) => Number(result[key])).filter(Number.isFinite);
      return {
        metric,
        p50_ms: nearestRank(values, 50)?.toFixed(2) ?? "N/A",
        p75_ms: nearestRank(values, 75)?.toFixed(2) ?? "N/A",
        p90_ms: nearestRank(values, 90)?.toFixed(2) ?? "N/A",
        p99_ms: nearestRank(values, 99)?.toFixed(2) ?? "N/A",
      };
    });
}

async function main() {
  const action = process.argv[2] || "status";
  const state = loadConfig();
  const routerApi = require("./router_api");
  const result = await routerApi.run(action, state, process.argv.slice(3));
  if (result === null || result === undefined) return;
  if (action === "start") {
    console.log(`Run ${result.run_id} | attempted ${result.attempted} | succeeded ${result.succeeded} | failed ${result.failed}`);
    console.table(latencySummary(result.results || []));
    return;
  }
  console.log(typeof result === "string" ? result : JSON.stringify(result, null, 2));
}

main().catch((error) => {
  console.error(`error: ${error.message}`);
  process.exit(1);
});
