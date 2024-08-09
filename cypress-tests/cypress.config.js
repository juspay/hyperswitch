const { defineConfig } = require("cypress");
const fs = require("fs-extra");
const path = require("path");

let globalState;
// Fetch from environment variable
const connectorId = process.env.CYPRESS_CONNECTOR;
const reportName = process.env.REPORT_NAME || `${connectorId}_report`;

module.exports = defineConfig({
  e2e: {
    setupNodeEvents(on, config) {
      require("cypress-mochawesome-reporter/plugin")(on);

      on("task", {
        setGlobalState: (val) => {
          return (globalState = val || {});
        },
        getGlobalState: () => {
          return globalState || {};
        },
        cli_log: (message) => {
          console.log("Logging console message from task");
          console.log(message);
          return null;
        },
      });
      on("after:screenshot", (details) => {
        // Full path to the screenshot file
        const screenshotPath = details.path;

        // Extract filename without extension
        const name = path.basename(
          screenshotPath,
          path.extname(screenshotPath)
        );

        // Define a new name with a connectorId
        const newName = `[${connectorId}] ${name}.png`;
        const newPath = path.join(path.dirname(screenshotPath), newName);

        return fs
          .rename(screenshotPath, newPath)
          .then(() => {
            console.log("Screenshot renamed successfully");
            return { path: newPath };
          })
          .catch((err) => {
            console.error("Failed to rename screenshot:", err);
          });
      });
    },
    experimentalRunAllSpecs: true,

    reporter: "cypress-mochawesome-reporter",
    reporterOptions: {
      reportDir: "cypress/reports",
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
});
