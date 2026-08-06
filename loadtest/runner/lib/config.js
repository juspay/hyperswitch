"use strict";

const fs = require("fs");
const path = require("path");
const { loadYaml, resolveMaybe } = require("../../lib/config");
const { buildPlan } = require("./scenarios");

const ROOT = path.resolve(__dirname, "../..");

function positive(value, name, allowZero = false) {
  const number = Number(value);
  if (!Number.isFinite(number) || (allowZero ? number < 0 : number <= 0)) {
    throw new Error(`${name} must be ${allowZero ? "non-negative" : "positive"}`);
  }
  return number;
}

function validate(config) {
  const plan = buildPlan(config.scenario || {});
  if (!config.services.router) throw new Error("deployment application_services.router.base_url is required");
  if (!config.services["modular-pm"]) throw new Error("deployment application_services.modular-pm.base_url is required");
  if (plan.usesPmService && !config.services.superposition) {
    throw new Error("deployment application_services.superposition.base_url is required for modular scenarios");
  }
  const load = config.load || {};
  const start = positive(load.starting_rps, "load.starting_rps");
  const target = positive(load.target_rps, "load.target_rps");
  const step = positive(load.step_rps, "load.step_rps", true);
  positive(load.hold_seconds, "load.hold_seconds");
  positive(load.idle_seconds, "load.idle_seconds", true);
  if (start > target) throw new Error("load.starting_rps cannot exceed load.target_rps");
  if (start < target && step === 0) throw new Error("load.step_rps must be positive when ramping");
  positive(config.fixtures?.concurrency, "fixtures.concurrency");
}

function loadConfig() {
  const requested = path.resolve(ROOT, process.env.CONFIG || "runner/config.yaml");
  const configPath = fs.existsSync(requested) ? requested : path.resolve(ROOT, "runner/config.example.yaml");
  const configDir = path.dirname(configPath);
  const config = loadYaml(configPath);
  const deploymentPath = resolveMaybe(configDir, config.deployment_config);
  const deployment = loadYaml(deploymentPath);
  const apps = deployment.application_services || {};
  const observability = deployment.observability || {};
  const grafana = observability.grafana_url?.replace(/\/$/, "");
  config.services = {
    router: apps.router?.base_url,
    "modular-pm": apps["modular-pm"]?.base_url,
    superposition: apps.superposition?.base_url,
    superposition_org_id: apps.superposition?.org_id,
    superposition_workspace_id: apps.superposition?.workspace_id,
    superposition_secret: apps.superposition?.secret,
    superposition_propagation_wait_seconds: apps.superposition?.propagation_wait_seconds,
    grafana,
    grafana_dashboard: grafana
      ? `${grafana}/d/${observability.grafana_dashboard_uid}/${observability.grafana_dashboard_slug}?orgId=1&from=now-15m&to=now&refresh=5s`
      : null,
  };
  validate(config);
  return { config, deployment, deploymentPath, configPath, configDir };
}

module.exports = { loadConfig };
