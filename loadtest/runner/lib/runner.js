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
  config.services = {
    payments: applications.payments?.base_url,
    modular: applications.modular?.base_url,
    superposition: superposition.base_url,
    superposition_org_id: superposition.org_id,
    superposition_workspace_id: superposition.workspace_id,
    superposition_secret: superposition.secret,
    superposition_propagation_wait_seconds: superposition.propagation_wait_seconds,
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
  if (!config.services.payments) throw new Error("deployment application_services.payments.base_url is required");
  if (!config.services.modular) throw new Error("deployment application_services.modular.base_url is required");
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

async function main() {
  const action = process.argv[2] || "status";
  const state = loadConfig();
  const paymentsApi = require("./payments_api");
  const result = await paymentsApi.run(action, state, process.argv.slice(3));
  if (result !== null && result !== undefined) console.log(typeof result === "string" ? result : JSON.stringify(result, null, 2));
}

main().catch((error) => {
  console.error(`error: ${error.message}`);
  process.exit(1);
});
