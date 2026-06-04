import { defineConfig } from "cypress";
import mochawesome from "cypress-mochawesome-reporter/plugin.js";
import crypto from "crypto";
import fs from "fs";
import https from "https";
import { getTimeoutMultiplier } from "./cypress/utils/RequestBodyUtils.js";

let globalState;

// Fetch from environment variable
const connectorId = process.env.CYPRESS_CONNECTOR || "service";
const screenshotsFolderName = `screenshots/${connectorId}`;
const reportName = process.env.REPORT_NAME || `${connectorId}_report`;
const retries = process.env.CYPRESS_MOCK_SERVER === "true" ? 0 : 2;

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
        verifyStripeAchPaymentIntent: ({
          connectorAuthFilePath,
          connectorId,
          paymentIntentId,
        }) => {
          const authContent = JSON.parse(
            fs.readFileSync(connectorAuthFilePath, "utf8")
          );
          const connectorData = authContent[connectorId];
          let apiKey;
          if (connectorData?.connector_account_details?.api_key) {
            apiKey = connectorData.connector_account_details.api_key;
          } else if (connectorData) {
            // MULTIPLE_CONNECTORS — get first entry's api_key
            const firstKey = Object.keys(connectorData)[0];
            apiKey =
              connectorData[firstKey]?.connector_account_details?.api_key;
          }
          if (!apiKey) {
            throw new Error(
              `Stripe API key not found in auth file for connector: ${connectorId}`
            );
          }

          const logs = [];

          const log = (msg) => {
            console.log(msg);
            logs.push(msg);
          };

          const stripeRequest = (method, path, postData) =>
            new Promise((resolve, reject) => {
              const headers = {
                Authorization: `Bearer ${apiKey}`,
                "Stripe-Version": "2023-10-16",
              };
              if (method === "POST") {
                headers["Content-Type"] = "application/x-www-form-urlencoded";
                headers["Content-Length"] = Buffer.byteLength(postData || "");
              }
              const options = {
                hostname: "api.stripe.com",
                path,
                method,
                headers,
              };
              const req = https.request(options, (res) => {
                let data = "";
                res.on("data", (chunk) => {
                  data += chunk;
                });
                res.on("end", () => {
                  try {
                    resolve({ status: res.statusCode, body: JSON.parse(data) });
                  } catch {
                    resolve({ status: res.statusCode, body: data });
                  }
                });
              });
              req.on("error", reject);
              if (method === "POST" && postData) req.write(postData);
              req.end();
            });

          log(`[stripeVerify] Using connectorId=${connectorId}`);

          // GET the PaymentIntent to extract the hosted_verification_url for microdeposit
          return stripeRequest(
            "GET",
            `/v1/payment_intents/${paymentIntentId}`,
            null
          ).then((piResult) => {
            log(
              `[stripeVerify] GET PI ${paymentIntentId} => ${piResult.status}`
            );
            if (piResult.status !== 200) {
              log(
                `[stripeVerify] GET PI failed: ${JSON.stringify(piResult.body)}`
              );
              return { status: piResult.status, body: piResult.body, logs };
            }
            const hostedVerificationUrl =
              piResult.body.next_action?.verify_with_microdeposits
                ?.hosted_verification_url;
            if (!hostedVerificationUrl) {
              log(
                "[stripeVerify] No hosted_verification_url — PI may already be verified"
              );
              return { status: 200, body: { already_verified: true }, logs };
            }
            log(
              `[stripeVerify] Got hosted_verification_url: ${hostedVerificationUrl}`
            );
            // Return the URL so the Cypress command can visit it via cy.origin()
            return {
              status: 200,
              body: { hosted_verification_url: hostedVerificationUrl },
              logs,
            };
          });
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
    retries: retries,
    video: true,
    videoCompression: 32,
    videosFolder: `cypress/videos/${connectorId}`,
    chromeWebSecurity: false,
  },
});
