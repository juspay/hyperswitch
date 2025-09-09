import { defineConfig } from "cypress";
import mochawesome from "cypress-mochawesome-reporter/plugin.js";
import fs from "fs";
import { getTimeoutMultiplier } from "./cypress/utils/RequestBodyUtils.js";

let globalState;

// Fetch from environment variable
const connectorId = process.env.CYPRESS_CONNECTOR || "service";
const screenshotsFolderName = `screenshots/${connectorId}`;
const reportName = process.env.REPORT_NAME || `${connectorId}_report`;

// Get timeout multiplier from shared utility
const timeoutMultiplier = getTimeoutMultiplier();

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
    video: true,
    videoCompression: 32,
    videosFolder: `cypress/videos/${connectorId}`,
    chromeWebSecurity: false,
  },
});
