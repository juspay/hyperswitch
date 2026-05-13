import { defineConfig } from "cypress";
import mochawesome from "cypress-mochawesome-reporter/plugin.js";
import crypto from "crypto";
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
        // HMAC-SHA256(secret, "{timestamp}.{body}") returned as hex.
        // Matches Stripe's webhook signing scheme (Stripe-Signature: t=ts,v1=hex).
        // Lives Node-side because Cypress's browser context lacks node:crypto.
        signStripeWebhook: ({ secret, timestamp, body }) => {
          return crypto
            .createHmac("sha256", secret)
            .update(`${timestamp}.${body}`)
            .digest("hex");
        },
        // Adyen's HMAC-SHA256 is different from Stripe's:
        //   - Key is *hex-decoded* (Adyen stores it as a hex string, Stripe uses raw)
        //   - Message is a colon-delimited string of 7 body fields, NOT timestamp.body
        //   - Output is base64, NOT hex
        //   - Signature goes inside the JSON body at
        //     notificationItems[0].NotificationRequestItem.additionalData.hmacSignature,
        //     not in a request header
        signAdyenWebhook: ({ secretHex, message }) => {
          return crypto
            .createHmac("sha256", Buffer.from(secretHex, "hex"))
            .update(message)
            .digest("base64");
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
    // Retries break the per-test step counter used by the MITM proxy
    // wrapper in support/e2e.js — a retry would re-enter beforeEach,
    // reset the counter, and re-issue cassette IDs out of sync with the
    // recorded ones. Keep retries off.
    retries: 0,

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
