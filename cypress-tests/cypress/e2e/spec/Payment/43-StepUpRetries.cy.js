import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Step-Up Retry Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Step-Up Retry Enabled - Payment fails without 3DS and retries with 3DS",
    () => {
      it("Setup Step-Up Config -> Create Payment Intent -> Confirm Payment without 3DS -> Verify Step-Up Retry with 3DS -> Retrieve Payment", () => {
        let shouldContinue = true;
        const connectorId = globalState.get("connectorId");

        cy.step("Check if connector supports Step-Up retry", () => {
          if (
            utils.shouldIncludeConnector(
              connectorId,
              utils.CONNECTOR_LISTS.INCLUDE.STEP_UP_RETRY
            )
          ) {
            cy.log(
              `Skipping Step-Up Retry - connector not supported: ${connectorId}`
            );
            shouldContinue = false;
            return;
          }
        });

        cy.step("Setup Step-Up enabled configuration", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Setup Step-Up Config");
            return;
          }

          const merchantId = globalState.get("merchantId");
          const key = `step_up_enabled_${merchantId}`;
          const value = JSON.stringify([connectorId]);

          cy.setConfigs(globalState, key, value, "CREATE");

          // Set flag indicating Step-Up retry is enabled for assertion validation
          globalState.set("isStepUpRetryEnabled", true);

          // For Step-Up retry, set secondary connector to same as primary
          // since retry happens on the same connector with 3DS
          globalState.set("secondaryConnector", connectorId);
        });

        cy.step("Enable auto retries for business profile", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Enable auto retries");
            return;
          }

          const updateBusinessProfileBody = {
            is_auto_retries_enabled: true,
            max_auto_retries_enabled: 1,
          };

          cy.UpdateBusinessProfileTest(
            updateBusinessProfileBody,
            false, // is_connector_agnostic_enabled
            false, // collect_billing_address_from_wallet_connector
            false, // collect_shipping_address_from_wallet_connector
            false, // always_collect_billing_address_from_wallet_connector
            false, // always_collect_shipping_address_from_wallet_connector
            globalState
          );

          globalState.set("max_auto_retries_enabled", 1);
        });

        cy.step("Create Payment Intent", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create Payment Intent");
            return;
          }

          const data =
            getConnectorDetails(connectorId)["card_pm"]["PaymentIntent"];

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

        cy.step("Confirm Payment without 3DS to trigger Step-Up retry", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }

          const data =
            getConnectorDetails(connectorId)["card_pm"]["No3DSFailPayment"];

          cy.confirmCallAutoRetryTest(
            fixtures.confirmBody,
            data,
            true,
            globalState
          );
        });

        cy.step("Retrieve Payment and verify Step-Up retry attempts", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }

          const data =
            getConnectorDetails(connectorId)["card_pm"]["No3DSFailPayment"];

          cy.retrievePaymentCallAutoRetryTest({
            globalState,
            data,
          });
        });
      });
    }
  );

  context("Step-Up Retry Disabled - Payment fails without retry", () => {
    it("Disable Step-Up Config -> Create Payment Intent -> Confirm Payment without 3DS -> Verify no retry occurred -> Retrieve Payment", () => {
      let shouldContinue = true;
      const connectorId = globalState.get("connectorId");

      cy.step("Check if connector supports Step-Up retry", () => {
        if (
          utils.shouldIncludeConnector(
            connectorId,
            utils.CONNECTOR_LISTS.INCLUDE.STEP_UP_RETRY
          )
        ) {
          cy.log(
            `Skipping Step-Up Retry Disabled test - connector not supported: ${connectorId}`
          );
          shouldContinue = false;
          return;
        }
      });

      cy.step("Ensure Step-Up is disabled", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Disable Step-Up Config");
          return;
        }

        const merchantId = globalState.get("merchantId");
        const key = `step_up_enabled_${merchantId}`;

        // Delete or set empty config to disable Step-Up
        cy.setConfigs(globalState, key, "[]", "UPDATE");

        // Set flag indicating Step-Up retry is disabled
        globalState.set("isStepUpRetryEnabled", false);

        // Clear secondary connector since we're not doing retries
        globalState.set("secondaryConnector", null);
      });

      cy.step("Update Business Profile - disable auto retries", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Update Business Profile");
          return;
        }

        const updateBusinessProfileBody = {
          is_auto_retries_enabled: false,
          max_auto_retries_enabled: 0,
        };

        cy.UpdateBusinessProfileTest(
          updateBusinessProfileBody,
          false, // is_connector_agnostic_enabled
          false, // collect_billing_address_from_wallet_connector
          false, // collect_shipping_address_from_wallet_connector
          false, // always_collect_billing_address_from_wallet_connector
          false, // always_collect_shipping_address_from_wallet_connector
          globalState
        );

        globalState.set("max_auto_retries_enabled", 0);
      });

      cy.step("Create Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Payment Intent");
          return;
        }

        const data =
          getConnectorDetails(connectorId)["card_pm"]["PaymentIntent"];

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

      cy.step("Confirm Payment without 3DS - should fail without retry", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }

        const data =
          getConnectorDetails(connectorId)["card_pm"]["No3DSFailPayment"];

        cy.confirmCallAutoRetryTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
      });

      cy.step("Retrieve Payment and verify no retry occurred", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }

        const data =
          getConnectorDetails(connectorId)["card_pm"]["No3DSFailPayment"];

        // Verify that no retry occurred - only 1 attempt, final status failed
        cy.retrievePaymentCallAutoRetryTest({
          globalState,
          data,
        });
      });
    });
  });
});
