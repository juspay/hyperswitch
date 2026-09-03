import { defineConfig } from "cypress";
import mochawesome from "cypress-mochawesome-reporter/plugin.js";
import crypto from "crypto";
import fs from "fs";
import { getTimeoutMultiplier } from "./cypress/utils/RequestBodyUtils.js";
import { multiplexLifecycleEvents } from "./cypress/utils/pluginEvents.js";
import { registerSpecTimings } from "./cypress/utils/specTimings.js";

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

// Get timeout multiplier from shared utility
const timeoutMultiplier = getTimeoutMultiplier();

export default defineConfig({
  env: forwardedEnv,
  e2e: {
    setupNodeEvents(on, config) {
      // Cypress keeps one handler per event, so every lifecycle listener below
      // — the reporter's, the timing report's and this file's own — has to be
      // registered through the multiplexer or the last one silently wins.
      const onEvent = multiplexLifecycleEvents(on);

      // Timings register first so the breakdown still reaches the log if the
      // reporter's own `after:run` throws while generating the report.
      registerSpecTimings(onEvent);
      mochawesome(onEvent);

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
      return config;
    },
    experimentalRunAllSpecs: true,

    specPattern: "cypress/e2e/**/*.cy.{js,jsx,ts,tsx}",
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
    video: false,
    chromeWebSecurity: false,
  },
});
