import { defineConfig } from "cypress";
import mochawesome from "cypress-mochawesome-reporter/plugin.js";
import fs from "fs";

let globalState;

// Fetch from environment variable
const connectorId = process.env.CYPRESS_CONNECTOR || "service";
const screenshotsFolderName = `screenshots/${connectorId}`;
const reportName = process.env.REPORT_NAME || `${connectorId}_report`;

export default defineConfig({
  e2e: {
    setupNodeEvents(on, config) {
      mochawesome(on);

      on("task", {
        setGlobalState: (val) => {
          return (globalState = val || {});
        },
        getGlobalState: () => {
          return globalState || {};
        },
        cli_log: (message) => {
          // eslint-disable-next-line no-console
          console.log("Logging console message from task");
          // eslint-disable-next-line no-console
          console.log(message);
          return null;
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
          // Delete video for passed specs
          fs.unlinkSync(results.video);
        }
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
    defaultCommandTimeout: 10000,
    pageLoadTimeout: 20000,
    responseTimeout: 30000,
    screenshotsFolder: screenshotsFolderName,
    video: true,
    videoCompression: 32,
    chromeWebSecurity: false,
    // TODO: Plan migration strategy for v15 when injectDocumentDomain is removed
    // Options:
    // 1. Restructure tests to avoid cross-subdomain testing
    // 2. Use alternative domain handling approaches
    // 3. Keep using v14 if cross-subdomain testing is critical
    injectDocumentDomain: true,
  },
});
