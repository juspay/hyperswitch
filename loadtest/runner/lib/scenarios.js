"use strict";

const MERCHANT_PATHS = new Set(["non_modular", "modular"]);

const SCENARIOS = Object.freeze({
  guest: Object.freeze({
    requiresCustomer: false,
    setupFutureUsage: null,
    storageType: "volatile",
  }),
  cit_on_session: Object.freeze({
    requiresCustomer: true,
    setupFutureUsage: "on_session",
    storageType: "persistent",
  }),
  cit_off_session: Object.freeze({
    requiresCustomer: true,
    setupFutureUsage: "off_session",
    storageType: "persistent",
  }),
  cit_metadata_changed: Object.freeze({
    requiresCustomer: true,
    requiresSavedCard: true,
    metadataChanged: true,
    setupFutureUsage: "off_session",
    storageType: "persistent",
  }),
});

function buildPlan(config) {
  const merchantPath = String(config.merchant_path || "non_modular").toLowerCase();
  const scenarioName = String(config.name || "cit_off_session").toLowerCase();
  if (!MERCHANT_PATHS.has(merchantPath)) throw new Error(`Unsupported merchant_path: ${merchantPath}`);
  const scenario = SCENARIOS[scenarioName];
  if (!scenario) throw new Error(`Unsupported scenario: ${scenarioName}`);
  if (scenario.metadataChanged && merchantPath !== "non_modular") {
    throw new Error("cit_metadata_changed supports only the non_modular merchant path");
  }
  return Object.freeze({
    id: `${merchantPath}:${scenarioName}`,
    merchantPath,
    scenarioName,
    usesPmService: merchantPath === "modular",
    ...scenario,
  });
}

module.exports = { MERCHANT_PATHS, SCENARIOS, buildPlan };
