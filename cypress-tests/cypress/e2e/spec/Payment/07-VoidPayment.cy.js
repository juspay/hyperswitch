import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";

let globalState;

describe("Card - NoThreeDS Manual payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card - void payment in Requires_capture state flow test", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Void Payment without Capture", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment after Confirmation", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Confirmation"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Void Payment without Capture", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Void Payment without Capture");
          return;
        }
        const voidData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["VoidAfterConfirm"];

        cy.voidCallTest(fixtures.voidBody, voidData, globalState);
      });
    });
  });

  context(
    "Card - void payment in Requires_payment_method state flow test",
    () => {
      it("Create Payment Intent -> Payment Methods Call -> Void Payment without Confirmation", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "manual",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Payment Methods Call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Payment Methods Call");
            return;
          }
          cy.paymentMethodsCallTest(globalState);
        });

        cy.step("Void Payment without Confirmation", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Void Payment without Confirmation"
            );
            return;
          }
          const voidData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Void"];

          cy.voidCallTest(fixtures.voidBody, voidData, globalState);
        });
      });
    }
  );

  context("Card - void payment in success state flow test", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Void Payment after Confirmation", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          false,
          globalState
        );

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment after Confirmation", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Confirmation"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Void Payment after Confirmation", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Void Payment after Confirmation");
          return;
        }
        const voidData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["VoidAfterConfirm"];

        cy.voidCallTest(fixtures.voidBody, voidData, globalState);
      });
    });
  });

  context("Card - sync a voided payment", () => {
    before("skip for connectors without void sync coverage", function () {
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.VOID_PAYMENT_SYNC
        )
      ) {
        this.skip();
      }
    });

    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Void Payment -> Retrieve Payment after Void", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Void Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Void Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["VoidAfterConfirm"];

        cy.voidCallTest(fixtures.voidBody, data, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment after Void", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment after Void");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["VoidAfterConfirm"];

        // `allKeysRequired: true` is needed here, not just `force_sync`: once
        // the attempt is already terminal (Voided), Hyperswitch's own
        // `should_call_connector` skips re-querying the connector, so a plain
        // force-synced retrieve would be served entirely from the DB and
        // never actually exercise Zift's `transactionType=void` sync
        // response — the exact thing PR #14637 fixes.
        cy.retrievePaymentCallTest({
          globalState,
          data,
          expectedIntentStatus: data.Response.body.status,
          allKeysRequired: true,
        });
      });
    });
  });
});
