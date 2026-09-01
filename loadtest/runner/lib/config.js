"use strict";

const fs = require("fs");
const path = require("path");
const { loadYaml, resolveMaybe } = require("../../lib/config");
const { buildPlan } = require("./scenarios");

const ROOT = path.resolve(__dirname, "../..");
const ENVIRONMENT_MODES = new Set(["local", "cloud"]);
const MERCHANT_MODES = new Set(["create", "existing"]);
const SUPERPOSITION_MODES = new Set(["manage", "preconfigured", "disabled"]);

function positive(value, name, allowZero = false) {
  const number = Number(value);
  if (!Number.isFinite(number) || (allowZero ? number < 0 : number <= 0)) {
    throw new Error(`${name} must be ${allowZero ? "non-negative" : "positive"}`);
  }
  return number;
}

function resolveConfiguredValue(section, key, env = process.env) {
  if (section?.[key] !== undefined && section[key] !== null && section[key] !== "") return section[key];
  const envName = section?.[`${key}_env`];
  if (!envName) return undefined;
  if (env[envName] === undefined || env[envName] === "") throw new Error(`Environment variable ${envName} is not set`);
  return env[envName];
}

function normalizePrefix(value, fallback = "") {
  const prefix = value === undefined || value === null ? fallback : String(value);
  if (!prefix || prefix === "/") return "";
  return `/${prefix.replace(/^\/+|\/+$/g, "")}`;
}

function joinUrl(baseUrl, suffix = "") {
  if (!baseUrl) return undefined;
  const base = String(baseUrl).replace(/\/+$/, "");
  if (!suffix) return base;
  if (/^https?:\/\//i.test(String(suffix))) return String(suffix).replace(/\/+$/, "");
  return `${base}/${String(suffix).replace(/^\/+/, "")}`.replace(/\/+$/, "");
}

function targetInput(config, name) {
  if (name === "modular-pm") return config.targets?.modular_pm || config.targets?.["modular-pm"] || {};
  return config.targets?.[name] || {};
}

function targetHeaders(input, env) {
  const headers = { ...(input.headers || {}) };
  for (const [header, envName] of Object.entries(input.headers_from_env || {})) {
    if (env[envName] === undefined || env[envName] === "") throw new Error(`Environment variable ${envName} is not set`);
    headers[header] = env[envName];
  }
  return headers;
}

function resolveTarget(config, deployment, name, env) {
  const apps = deployment.application_services || {};
  const app = apps[name] || {};
  const input = targetInput(config, name);
  const defaultPrefix = name === "modular-pm" ? "/v2" : "";
  const rawBaseUrl = input.base_url || app.base_url;
  const apiPrefix = normalizePrefix(input.api_prefix, defaultPrefix);
  return {
    base_url: joinUrl(rawBaseUrl, apiPrefix),
    raw_base_url: rawBaseUrl ? joinUrl(rawBaseUrl) : undefined,
    api_prefix: apiPrefix,
    health_url: input.health_url ? joinUrl(rawBaseUrl, input.health_url) : undefined,
    headers: targetHeaders(input, env),
    org_id: input.org_id || app.org_id,
    workspace_id: input.workspace_id || app.workspace_id,
    secret: resolveConfiguredValue(input, "secret", env) || app.secret,
    propagation_wait_seconds: input.propagation_wait_seconds ?? app.propagation_wait_seconds,
  };
}

function requiredTargetNames(plan, superpositionMode) {
  const names = ["router"];
  if (plan.usesPmService || plan.requiresCustomer) names.push("modular-pm");
  if (superpositionMode === "manage") names.push("superposition");
  return names;
}

function resolvedMerchant(config, plan, superpositionMode, env) {
  const merchantConfig = config.merchant || {};
  const mode = String(merchantConfig.mode || "create").toLowerCase();
  if (!MERCHANT_MODES.has(mode)) throw new Error("merchant.mode must be create or existing");
  if (mode !== "existing") return { mode, merchant: null };
  const merchant = {
    merchant_id: resolveConfiguredValue(merchantConfig, "merchant_id", env),
    merchant_api_key: resolveConfiguredValue(merchantConfig, "api_key", env),
    publishable_key: resolveConfiguredValue(merchantConfig, "publishable_key", env),
    profile_id: resolveConfiguredValue(merchantConfig, "profile_id", env),
    organization_id: resolveConfiguredValue(merchantConfig, "organization_id", env),
    merchant_connector_id: resolveConfiguredValue(merchantConfig, "merchant_connector_id", env),
    merchant_path: plan.merchantPath,
  };
  for (const key of ["merchant_id", "merchant_api_key", "profile_id"]) {
    if (!merchant[key]) throw new Error(`merchant.${key === "merchant_api_key" ? "api_key" : key} is required when merchant.mode=existing`);
  }
  if (plan.usesPmService && !merchant.publishable_key) {
    throw new Error("merchant.publishable_key is required for modular PM-session confirmation");
  }
  if (superpositionMode === "manage" && !merchant.organization_id) {
    throw new Error("merchant.organization_id is required when superposition.mode=manage");
  }
  return { mode, merchant };
}

function validateLoad(config) {
  const load = config.load || {};
  const start = positive(load.starting_rps, "load.starting_rps");
  const target = positive(load.target_rps, "load.target_rps");
  const step = positive(load.step_rps, "load.step_rps", true);
  positive(load.hold_seconds, "load.hold_seconds");
  positive(load.idle_seconds, "load.idle_seconds", true);
  if (start > target) throw new Error("load.starting_rps cannot exceed load.target_rps");
  if (start < target && step === 0) throw new Error("load.step_rps must be positive when ramping");
  positive(config.fixtures?.concurrency, "fixtures.concurrency");
  positive(config.fixtures?.wait_before_start_seconds ?? 0, "fixtures.wait_before_start_seconds", true);
}

function resolveRunnerConfiguration(config, deployment = {}, env = process.env) {
  const plan = buildPlan(config.scenario || {});
  const environmentMode = String(config.environment?.mode || "local").toLowerCase();
  if (!ENVIRONMENT_MODES.has(environmentMode)) throw new Error("environment.mode must be local or cloud");
  const superpositionMode = String(
    config.superposition?.mode || (environmentMode === "cloud" ? "preconfigured" : "manage"),
  ).toLowerCase();
  if (!SUPERPOSITION_MODES.has(superpositionMode)) {
    throw new Error("superposition.mode must be manage, preconfigured, or disabled");
  }
  const targets = {
    router: resolveTarget(config, deployment, "router", env),
    "modular-pm": resolveTarget(config, deployment, "modular-pm", env),
    superposition: resolveTarget(config, deployment, "superposition", env),
  };
  for (const name of requiredTargetNames(plan, superpositionMode)) {
    if (!targets[name].base_url) throw new Error(`${name} target base_url is required for ${plan.id}`);
    if (environmentMode === "cloud" && config.preflight?.probe_targets !== false && !targets[name].health_url) {
      throw new Error(`${name} target health_url is required for cloud preflight`);
    }
  }
  validateLoad(config);
  const merchant = resolvedMerchant(config, plan, superpositionMode, env);
  if (environmentMode === "cloud" && merchant.mode === "create") {
    const adminApiKey = resolveConfiguredValue(config.payments_api || {}, "admin_api_key", env);
    if (!adminApiKey) throw new Error("payments_api.admin_api_key or admin_api_key_env is required for cloud merchant creation");
  }
  const deploymentObservability = deployment.observability || {};
  const observability = { ...deploymentObservability, ...(config.observability || {}) };
  const grafana = observability.grafana_url?.replace(/\/$/, "");
  const services = {
    router: targets.router.base_url,
    "modular-pm": targets["modular-pm"].base_url,
    superposition: targets.superposition.base_url,
    grafana,
    grafana_dashboard: grafana && observability.grafana_dashboard_uid && observability.grafana_dashboard_slug
      ? `${grafana}/d/${observability.grafana_dashboard_uid}/${observability.grafana_dashboard_slug}?orgId=1&from=now-15m&to=now&refresh=5s`
      : grafana,
  };
  return {
    ...config,
    environment_mode: environmentMode,
    targets,
    target_headers: Object.fromEntries(Object.entries(targets).map(([name, target]) => [name, target.headers])),
    services,
    superposition: { ...(config.superposition || {}), mode: superpositionMode },
    merchant: { ...(config.merchant || {}), mode: merchant.mode },
    resolved_merchant: merchant.merchant,
    required_targets: requiredTargetNames(plan, superpositionMode),
    observability,
  };
}

function loadConfig() {
  const explicitConfig = process.env.CONFIG;
  const requested = path.resolve(ROOT, explicitConfig || "runner/config.yaml");
  if (explicitConfig && !fs.existsSync(requested)) throw new Error(`Config file not found: ${requested}`);
  const configPath = fs.existsSync(requested) ? requested : path.resolve(ROOT, "runner/config.example.yaml");
  const configDir = path.dirname(configPath);
  const sourceConfig = loadYaml(configPath);
  const deploymentPath = sourceConfig.deployment_config
    ? resolveMaybe(configDir, sourceConfig.deployment_config)
    : null;
  if (String(sourceConfig.environment?.mode || "local").toLowerCase() === "local" && !deploymentPath) {
    throw new Error("deployment_config is required when environment.mode=local");
  }
  const deployment = deploymentPath ? loadYaml(deploymentPath) : {};
  const config = resolveRunnerConfiguration(sourceConfig, deployment);
  return { config, deployment, deploymentPath, configPath, configDir };
}

module.exports = {
  joinUrl,
  loadConfig,
  requiredTargetNames,
  resolveConfiguredValue,
  resolveRunnerConfiguration,
};
