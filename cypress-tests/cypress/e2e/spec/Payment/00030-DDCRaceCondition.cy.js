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
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("[Payment] DDC Race Condition Tests", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("[Payment] Server-side DDC race condition handling", () => {
      const createData = getConnectorDetails(connector)["card_pm"]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        createData,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue)
        shouldContinue = utils.should_continue_further(createData);

      const confirmData = getConnectorDetails(connector)["card_pm"]["DDCRaceConditionServerSide"];

      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

      if (shouldContinue)
        shouldContinue = utils.should_continue_further(confirmData);

      const ddcConfig = confirmData.DDCConfig;
      const paymentId = globalState.get("paymentID");
      const merchantId = globalState.get("merchantId");
      const completeUrl = `${Cypress.env("BASEURL")}/payments/${paymentId}/${merchantId}${ddcConfig.completeUrlPath}`;

      cy.request({
        method: "GET",
        url: completeUrl,
        qs: {
          [ddcConfig.collectionReferenceParam]: ddcConfig.firstSubmissionValue,
        },
        failOnStatusCode: false,
      }).then((firstResponse) => {
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

    it("[Payment] Client-side DDC race condition handling", () => {
      const createData = getConnectorDetails(connector)["card_pm"]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        createData,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue)
        shouldContinue = utils.should_continue_further(createData);

      const confirmData = getConnectorDetails(connector)["card_pm"]["DDCRaceConditionClientSide"];

      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

      if (shouldContinue)
        shouldContinue = utils.should_continue_further(confirmData);

      const ddcConfig = confirmData.DDCConfig;
      const paymentId = globalState.get("paymentID");
      const merchantId = globalState.get("merchantId");
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
          interception.request.query[ddcConfig.collectionReferenceParam] || "";
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
