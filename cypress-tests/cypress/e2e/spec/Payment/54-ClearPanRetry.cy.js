import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Clear PAN Retry Tests", function () {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connectorId = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connectorId,
            utils.CONNECTOR_LISTS.INCLUDE.CLEAR_PAN_RETRY
          )
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

  context("clear-pan-retries-enabled", () => {
    it("Enable Clear PAN Retry -> Create Payment Intent -> Confirm Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Update Business Profile to enable clear PAN retries", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Update Business Profile");
          return;
        }

        const updateBusinessProfileBody = {
          is_auto_retries_enabled: true,
          is_network_tokenization_enabled: true,
          max_auto_retries_enabled: 2,
          is_clear_pan_retries_enabled: true,
        };

        cy.UpdateBusinessProfileTest(
          updateBusinessProfileBody,
          /* is_connector_agnostic_mit_enabled */ false,
          /* collect_billing_details_from_wallet_connector */ false,
          /* collect_shipping_details_from_wallet_connector */ false,
          /* always_collect_billing_details_from_wallet_connector */ false,
          /* always_collect_shipping_details_from_wallet_connector */ false,
          globalState
        );

        globalState.set("max_auto_retries_enabled", 2);
      });

      cy.step("Create Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Payment Intent");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment and verify attempts", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }

        // NOTE: skipRetryAssertion=true because the sandbox lacks a connector that supports
        // PaymentMethodData::NetworkToken, so the attempts.length > 1 assertion is skipped.
        // This step only verifies the payment has attempts with valid attempt_id/connector fields.
        // When a sandbox connector supports clear PAN retry, remove skipRetryAssertion and run the full assertion.
        cy.retrievePaymentCallClearPanRetryTest({
          globalState,
          isClearPanRetryEnabled: true,
          skipRetryAssertion: true,
        });
      });
    });
  });

  context("clear-pan-retries-disabled", () => {
    it("Disable Clear PAN Retry -> Create Payment Intent -> Confirm Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Update Business Profile to disable clear PAN retries", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Update Business Profile");
          return;
        }

        const updateBusinessProfileBody = {
          is_auto_retries_enabled: true,
          is_network_tokenization_enabled: true,
          max_auto_retries_enabled: 2,
          is_clear_pan_retries_enabled: false,
        };

        cy.UpdateBusinessProfileTest(
          updateBusinessProfileBody,
          /* is_connector_agnostic_mit_enabled */ false,
          /* collect_billing_details_from_wallet_connector */ false,
          /* collect_shipping_details_from_wallet_connector */ false,
          /* always_collect_billing_details_from_wallet_connector */ false,
          /* always_collect_shipping_details_from_wallet_connector */ false,
          globalState
        );

        globalState.set("max_auto_retries_enabled", 2);
      });

      cy.step("Create Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Payment Intent");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment and verify attempts", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }

        cy.retrievePaymentCallClearPanRetryTest({
          globalState,
          isClearPanRetryEnabled: false,
        });
      });
    });
  });

  context("reset-business-profile", () => {
    it("Reset business profile to disable clear PAN retries", () => {
      cy.step("Reset business profile flags", () => {
        const updateBusinessProfileBody = {
          is_auto_retries_enabled: false,
          is_network_tokenization_enabled: false,
          max_auto_retries_enabled: 0,
          is_clear_pan_retries_enabled: false,
        };

        cy.UpdateBusinessProfileTest(
          updateBusinessProfileBody,
          /* is_connector_agnostic_mit_enabled */ false,
          /* collect_billing_details_from_wallet_connector */ false,
          /* collect_shipping_details_from_wallet_connector */ false,
          /* always_collect_billing_details_from_wallet_connector */ false,
          /* always_collect_shipping_details_from_wallet_connector */ false,
          globalState
        );
      });
    });
  });
});
