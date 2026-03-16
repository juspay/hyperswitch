import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Auto Retry Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("auto retries enabled with max retries = 1", () => {
    it("Create Secondary Connector -> Update Business Profile -> Create Payment Intent -> Confirm Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Secondary Connector", () => {
        const CONNECTOR_POOL = ["stripe", "adyen", "cybersource"];
        const primaryConnector = globalState.get("connectorId");

        const secondaryConnector = CONNECTOR_POOL.find(
          (connector) => connector !== primaryConnector
        );

        globalState.set("connectorId", secondaryConnector);
        globalState.set("secondaryConnector", secondaryConnector);

        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState,
          "profile",
          "merchantConnectorSecondary"
        ).then(() => {
          globalState.set("connectorId", primaryConnector);
        });
      });

      cy.step(
        "Update Business Profile to enable auto retries with 1 max retry",
        () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Update Business Profile");
            return;
          }

          const connectorId = globalState.get("connectorId");
          if (
            utils.shouldIncludeConnector(
              connectorId,
              utils.CONNECTOR_LISTS.INCLUDE.AUTO_RETRY
            )
          ) {
            cy.log(
              `Skipping Auto Retry - connector not supported: ${connectorId}`
            );
            shouldContinue = false;
            return;
          }

          const updateBusinessProfileBody = {
            is_auto_retries_enabled: true,
            max_auto_retries_enabled: 1,
          };

          cy.UpdateBusinessProfileTest(
            updateBusinessProfileBody,
            false,
            false,
            false,
            false,
            false,
            globalState
          );

          globalState.set("max_auto_retries_enabled", 1);
        }
      );

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

        const activeConnector = globalState.get("connectorId");

        const data =
          getConnectorDetails(activeConnector)["card_pm"]["No3DSFailPayment"];

        cy.confirmCallAutoRetryTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSFailPayment"];

        cy.retrievePaymentCallAutoRetryTest({ globalState, data });
      });
    });
  });

  context("auto retries enabled with max retries = 0", () => {
    it("Update Business Profile -> Create Payment Intent -> Confirm Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step(
        "Update Business Profile to enable auto retries with 0 max retries",
        () => {
          const connectorId = globalState.get("connectorId");
          if (
            utils.shouldIncludeConnector(
              connectorId,
              utils.CONNECTOR_LISTS.INCLUDE.AUTO_RETRY
            )
          ) {
            cy.log(
              `Skipping Auto Retry - connector not supported: ${connectorId}`
            );
            shouldContinue = false;
            return;
          }

          const body = {
            is_auto_retries_enabled: true,
            max_auto_retries_enabled: 0,
          };

          cy.UpdateBusinessProfileTest(
            body,
            false,
            false,
            false,
            false,
            false,
            globalState
          );

          globalState.set("max_auto_retries_enabled", 0);
        }
      );

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

        const activeConnector = globalState.get("connectorId");

        const data =
          getConnectorDetails(activeConnector)["card_pm"]["No3DSFailPayment"];

        cy.confirmCallAutoRetryTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSFailPayment"];

        cy.retrievePaymentCallAutoRetryTest({ globalState, data });
      });
    });
  });
});
