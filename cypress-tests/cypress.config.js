import { defineConfig } from "cypress";
import "cypress-mochawesome-reporter/plugin.js";

let globalState;

// Fetch from environment variable
const connectorId = process.env.CYPRESS_CONNECTOR || "service";
const screenshotsFolderName = `screenshots/${connectorId}`;
const reportName = process.env.REPORT_NAME || `${connectorId}_report`;

export default defineConfig({
  e2e: {
    setupNodeEvents(on) {
      let sharedState = {};

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
        // Shared state makes it possible to share state / environment variables between tests
        // This is similar to globalState but much simpler
        setSharedState: (state) => {
          sharedState = { ...sharedState, ...state };
          return null;
        },
        getSharedState: () => {
          return sharedState;
        },
      });
    },
    experimentalRunAllSpecs: true,

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
  },
  chromeWebSecurity: false,
  defaultCommandTimeout: 10000,
  pageLoadTimeout: 20000,

  screenshotsFolder: screenshotsFolderName,
});
