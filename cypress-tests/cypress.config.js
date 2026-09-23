import { defineConfig } from "cypress";
import mochawesome from "cypress-mochawesome-reporter/plugin.js";
import crypto from "crypto";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "node:url";
import { getTimeoutMultiplier } from "./cypress/utils/RequestBodyUtils.js";

let globalState;

// Fetch from environment variable
const connectorId = process.env.CYPRESS_CONNECTOR || "service";
const screenshotsFolderName = `screenshots/${connectorId}`;
const reportName = process.env.REPORT_NAME || `${connectorId}_report`;
const retries = process.env.CYPRESS_MOCK_SERVER === "true" ? 0 : 2;

// Cypress only auto-maps `CYPRESS_` prefixed variables onto `Cypress.env()`, so
// these are forwarded explicitly and can be exported without the prefix. A
// CYPRESS_ prefixed variable still wins, since those override the config file.
// Names must match what `cypress/utils/State.js` reads.
const forwardedEnv = [
  "PM_SERVICE_URL",
  "SUPERPOSITION_BASE_URL",
  "SUPERPOSITION_SECRET",
  "SUPERPOSITION_API_KEY",
  "SUPERPOSITION_AUTH_TOKEN",
  "SUPERPOSITION_ORG_ID",
  "SUPERPOSITION_WORKSPACE_ID",
].reduce((acc, name) => {
  // Only forward what is actually set, so an absent variable never shadows a
  // CYPRESS_ prefixed one
  if (process.env[name] !== undefined) {
    acc[name] = process.env[name];
  }
  return acc;
}, {});

const superpositionEnvMapping = {
  SUPERPOSITION_BASE_URL: "endpoint",
  SUPERPOSITION_SECRET: "token",
  SUPERPOSITION_AUTH_TOKEN: "token",
  SUPERPOSITION_ORG_ID: "org_id",
  SUPERPOSITION_WORKSPACE_ID: "workspace_id",
};

const readTomlSection = (filePath, sectionName) => {
  const values = {};
  let contents;
  try {
    contents = fs.readFileSync(filePath, "utf8");
  } catch {
    return values;
  }

  let inSection = false;
  for (const line of contents.split("\n")) {
    const trimmed = line.trim();
    if (trimmed.startsWith("[")) {
      inSection = trimmed === `[${sectionName}]`;
      continue;
    }
    if (!inSection) {
      continue;
    }
    const entry = trimmed.match(/^([A-Za-z0-9_]+)\s*=\s*"((?:[^"\\]|\\.)*)"/);
    if (entry) {
      values[entry[1]] = entry[2].replace(/\\(["\\])/g, "$1");
    }
  }
  return values;
};

const isEnvSet = (name) =>
  process.env[name] !== undefined ||
  process.env[`CYPRESS_${name}`] !== undefined;

// development.toml mirrors the local router config, so the fallback below is
// local-only; remote targets front superposition next to the router API.
const routerTarget =
  process.env.CYPRESS_BASEURL || process.env.BASEURL || "http://localhost:8080";
const isLocalRouterTarget = (() => {
  try {
    const { hostname } = new URL(routerTarget);
    return hostname === "localhost" || hostname === "127.0.0.1";
  } catch {
    return true; // unparsable target → dev default
  }
})();

// https://integ.hyperswitch.io/api → https://integ.hyperswitch.io/superposition
const deriveSuperpositionBaseUrl = () => {
  try {
    const url = new URL(routerTarget);
    const apiPath = url.pathname.replace(/\/+$/, "");
    if (!/^(?:\/v\d+)?\/api$/i.test(apiPath)) {
      return undefined; // no /api mount — don't guess
    }
    return url.origin + apiPath.replace(/api$/i, "superposition");
  } catch {
    return undefined;
  }
};

const applySuperpositionFallback = async (config) => {
  const missing = Object.keys(superpositionEnvMapping).filter(
    (name) => !isEnvSet(name)
  );
  if (missing.length === 0) {
    return;
  }

  if (!isLocalRouterTarget) {
    // Derive only the endpoint for remote targets; never fabricate the secret.
    if (
      missing.includes("SUPERPOSITION_BASE_URL") &&
      config.env.SUPERPOSITION_BASE_URL === undefined
    ) {
      const derived = deriveSuperpositionBaseUrl();
      if (derived !== undefined) {
        // eslint-disable-next-line no-console
        console.log(
          `[cypress.config] Superposition endpoint derived from router target (${derived})`
        );
        config.env.SUPERPOSITION_BASE_URL = derived;
      }
    }
    return;
  }

  const tomlValues = readTomlSection(
    path.join(
      path.dirname(fileURLToPath(import.meta.url)),
      "..",
      "config",
      "development.toml"
    ),
    "superposition"
  );
  const resolved = {};
  for (const name of missing) {
    const value = tomlValues[superpositionEnvMapping[name]];
    if (value !== undefined && value !== "") {
      resolved[name] = value;
    }
  }

  if (Object.keys(resolved).length === 0) {
    return;
  }

  Object.assign(config.env, resolved);
  // eslint-disable-next-line no-console
  console.log(
    `[cypress.config] Superposition credentials resolved from config/development.toml`
  );
};

// Get timeout multiplier from shared utility
const timeoutMultiplier = getTimeoutMultiplier();

// Named ONLY_SPECS/SKIP_SPECS rather than SPEC_PATTERN/EXCLUDE_SPEC_PATTERN:
// Cypress auto-maps any CYPRESS_<X> env var directly onto the matching
// top-level config key (specPattern, excludeSpecPattern) before this file
// even runs, with the raw unsplit string, bypassing the split(",") below
// entirely — so the env var name must not collide with a real config key.
const excludeSpecPattern = process.env.CYPRESS_SKIP_SPECS
  ? process.env.CYPRESS_SKIP_SPECS.split(",")
  : [];

const specPattern = process.env.CYPRESS_ONLY_SPECS
  ? process.env.CYPRESS_ONLY_SPECS.split(",")
  : "cypress/e2e/**/*.cy.{js,jsx,ts,tsx}";

export default defineConfig({
  env: forwardedEnv,
  e2e: {
    async setupNodeEvents(on, config) {
      mochawesome(on);

      await applySuperpositionFallback(config);

      on("task", {
        setGlobalState: (val) => {
          return (globalState = val || {});
        },
        getGlobalState: () => {
          return globalState || {};
        },
        readFileOrNull: (filePath) => {
          if (!fs.existsSync(filePath)) return null;
          try {
            return JSON.parse(fs.readFileSync(filePath, "utf8"));
          } catch {
            return null;
          }
        },
        cli_log: (message) => {
          // eslint-disable-next-line no-console
          console.log("Logging console message from task");
          // eslint-disable-next-line no-console
          console.log(message);
          return null;
        },
        computeHmac: ({ key, message, algorithm = "sha512" }) => {
          if (!key || !message) {
            throw new Error(
              `computeHmac: 'key' and 'message' are required (got key=${!!key}, message=${!!message})`
            );
          }
          const signature = crypto
            .createHmac(algorithm, key)
            .update(message)
            .digest("hex");
          return signature;
        },
      });
      on("after:spec", (spec, results) => {
        // Clean up resources after each spec
        if (
          results &&
          results.video &&
          !results.tests.some((test) =>
            test.attempts.some((attempt) => attempt.state === "failed")
          )
        ) {
          // Only try to delete if the video file exists
          try {
            if (fs.existsSync(results.video)) {
              fs.unlinkSync(results.video);
            }
          } catch (error) {
            // Log the error but don't fail the test
            // eslint-disable-next-line no-console
            console.warn(
              `Warning: Could not delete video file: ${results.video}`
            );
            // eslint-disable-next-line no-console
            console.warn(error);
          }
        }
      });
      return config;
    },
    experimentalRunAllSpecs: true,

    specPattern,
    excludeSpecPattern,
    supportFile: "cypress/support/e2e.js",

    reporter: "cypress-mochawesome-reporter",
    reporterOptions: {
      reportDir: `cypress/reports/${connectorId}`,
      reportFilename: reportName,
      reportPageTitle: `[${connectorId}] Cypress test report`,
      embeddedScreenshots: true,
      overwrite: false,
      inlineAssets: true,
      saveJson: true,
    },
    defaultCommandTimeout: Math.round(30000 * timeoutMultiplier),
    pageLoadTimeout: Math.round(90000 * timeoutMultiplier), // 90s local, 135s (2.25min) CI
    responseTimeout: Math.round(60000 * timeoutMultiplier),
    requestTimeout: Math.round(45000 * timeoutMultiplier),
    taskTimeout: Math.round(120000 * timeoutMultiplier),
    screenshotsFolder: screenshotsFolderName,
    retries: retries,
    video: true,
    videoCompression: 32,
    videosFolder: `cypress/videos/${connectorId}`,
    chromeWebSecurity: false,
  },
});
