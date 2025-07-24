/**
 * DDC Race Condition Tests
 *
 * These tests ensure that device data collection works properly during payment authentication
 * and prevents issues when multiple requests happen at the same time.
 *
 * Server-side validation:
 * - Checks that our backend properly handles duplicate device data submissions
 * - Makes sure that once device data is collected, any additional attempts are rejected
 *
 * Client-side validation:
 * - Verifies that the payment page prevents users from accidentally submitting data twice
 * - Ensures that even if someone clicks multiple times, only one submission goes through
 * - Tests that our JavaScript protection works as expected
 */

import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let connector;
let globalState;

describe("[Payment] DDC Race Condition", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        const connectorDetails = getConnectorDetails(connector);
        if (
          !connectorDetails["card_pm"]["DDCRaceConditionServerSide"] ||
          !connectorDetails["card_pm"]["DDCRaceConditionClientSide"]
        ) {
          skip = true;
          return;
        }

        const requiredKeys = [
          "merchantId",
          "apiKey",
          "publishableKey",
          "baseUrl",
        ];
        const missingKeys = requiredKeys.filter((key) => !globalState.get(key));

        if (missingKeys.length > 0) {
          cy.log(
            `Skipping DDC tests - missing critical state: ${missingKeys.join(", ")}`
          );
          skip = true;
          return;
        }

        const merchantConnectorId = globalState.get("merchantConnectorId");
        if (!merchantConnectorId) {
          cy.log(
            "Warning: merchantConnectorId missing - may indicate connector configuration issue"
          );
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  afterEach("comprehensive cleanup", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("[Payment] DDC Race Condition Tests", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }

      // Only reset payment-specific state, don't clear paymentID here as it might be needed
      globalState.set("clientSecret", null);
      globalState.set("nextActionUrl", null);

      if (!globalState.get("customerId")) {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      }

      if (!globalState.get("profileId")) {
        const defaultProfileId = globalState.get("defaultProfileId");
        if (defaultProfileId) {
          globalState.set("profileId", defaultProfileId);
        }
      }
    });

    it("[Payment] Server-side DDC race condition handling", () => {
      const createData =
        getConnectorDetails(connector)["card_pm"]["PaymentIntent"];
      const confirmData =
        getConnectorDetails(connector)["card_pm"]["DDCRaceConditionServerSide"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        createData,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue)
        shouldContinue = utils.should_continue_further(createData);

      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

      if (shouldContinue)
        shouldContinue = utils.should_continue_further(confirmData);

      cy.then(() => {
        const ddcConfig = confirmData.DDCConfig;
        const paymentId = globalState.get("paymentID");
        const merchantId = globalState.get("merchantId");

        if (!merchantId) {
          throw new Error(
            `Missing merchantId - this indicates a critical state issue`
          );
        }

        if (!paymentId) {
          throw new Error(
            "Failed to create payment intent - paymentID not found in globalState"
          );
        }

        const completeUrl = `${Cypress.env("BASEURL")}/payments/${paymentId}/${merchantId}${ddcConfig.completeUrlPath}`;

        cy.request({
          method: "GET",
          url: completeUrl,
          qs: {
            [ddcConfig.collectionReferenceParam]:
              ddcConfig.firstSubmissionValue,
          },
          failOnStatusCode: false,
        }).then((firstResponse) => {
          if (
            firstResponse.status === 400 &&
            firstResponse.body?.error?.message?.includes(
              "No eligible connector"
            )
          ) {
            throw new Error(
              `Connector configuration issue detected. This may be due to state pollution from previous tests. Response: ${JSON.stringify(firstResponse.body)}`
            );
          }

          expect(firstResponse.status).to.be.oneOf([200, 302]);
          cy.log(`First request status: ${firstResponse.status}`);

          cy.request({
            method: "GET",
            url: completeUrl,
            qs: {
              [ddcConfig.collectionReferenceParam]:
                ddcConfig.secondSubmissionValue,
            },
            failOnStatusCode: false,
          }).then((secondResponse) => {
            cy.log(`Second request status: ${secondResponse.status}`);

            expect(secondResponse.status).to.eq(ddcConfig.expectedError.status);
            expect(secondResponse.body).to.deep.equal(
              ddcConfig.expectedError.body
            );

            cy.log(
              "✅ Server-side race condition protection verified - second submission properly rejected"
            );
          });
        });
      });
    });

    it("[Payment] Client-side DDC race condition handling", () => {
      const createData =
        getConnectorDetails(connector)["card_pm"]["PaymentIntent"];
      const confirmData =
        getConnectorDetails(connector)["card_pm"]["DDCRaceConditionClientSide"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        createData,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue)
        shouldContinue = utils.should_continue_further(createData);

      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

      if (shouldContinue)
        shouldContinue = utils.should_continue_further(confirmData);

      cy.then(() => {
        const ddcConfig = confirmData.DDCConfig;
        const paymentId = globalState.get("paymentID");
        const merchantId = globalState.get("merchantId");

        if (!merchantId) {
          throw new Error(
            `Missing merchantId - this indicates a critical state issue`
          );
        }

        if (!paymentId) {
          throw new Error(
            "Failed to create payment intent - paymentID not found in globalState"
          );
        }

        const nextActionUrl = `${Cypress.env("BASEURL")}${ddcConfig.redirectUrlPath}/${paymentId}/${merchantId}/${paymentId}_1`;

        cy.intercept("GET", nextActionUrl, (req) => {
          req.reply((res) => {
            let modifiedHtml = res.body.toString();
            modifiedHtml = modifiedHtml.replace(
              "</body>",
              ddcConfig.raceConditionScript + "</body>"
            );
            res.send(modifiedHtml);
          });
        }).as("ddcPageWithRaceCondition");

        cy.intercept("GET", "**/redirect/complete/**").as("ddcSubmission");

        cy.visit(nextActionUrl);
        cy.wait("@ddcPageWithRaceCondition");
        cy.wait("@ddcSubmission");
        cy.wait(2000);

        cy.get("@ddcSubmission.all").should("have.length", 1);

        cy.get("@ddcSubmission").then((interception) => {
          const collectionRef =
            interception.request.query[ddcConfig.collectionReferenceParam] ||
            "";
          cy.log(
            `Single submission detected with ${ddcConfig.collectionReferenceParam}: "${collectionRef}"`
          );
        });

        cy.log(
          "✅ Client-side race condition protection verified - only one submission occurred"
        );
      });
    });
  });
});
