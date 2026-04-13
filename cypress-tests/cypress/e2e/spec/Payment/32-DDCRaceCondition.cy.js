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
import getConnectorDetails, {
  shouldIncludeConnector,
  CONNECTOR_LISTS,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let connector;
let globalState;

describe("[Payment] DDC Race Condition", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.DDC_RACE_CONDITION
          )
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

      cy.ddcServerSideRaceConditionTest(confirmData, globalState);
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

      cy.ddcClientSideRaceConditionTest(confirmData, globalState);
    });
  });
});
