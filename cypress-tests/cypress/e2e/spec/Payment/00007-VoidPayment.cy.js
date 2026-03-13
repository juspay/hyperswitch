import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import reportErrors from "../../../utils/reportErrors";

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
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create Payment Intent", errorStack, () => {
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

      cy.step("Payment Methods Call", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment Intent", errorStack, () => {
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

      cy.step("Retrieve Payment after Confirmation", errorStack, () => {
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

      cy.step("Void Payment without Capture", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Void Payment without Capture");
          return;
        }
        const voidData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["VoidAfterConfirm"];

        cy.voidCallTest(fixtures.voidBody, voidData, globalState);
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context(
    "Card - void payment in Requires_payment_method state flow test",
    () => {
      it("Create Payment Intent -> Payment Methods Call -> Void Payment without Confirmation", () => {
        const errorStack = [];
        let shouldContinue = true;

        cy.step("Create Payment Intent", errorStack, () => {
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

        cy.step("Payment Methods Call", errorStack, () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Payment Methods Call");
            return;
          }
          cy.paymentMethodsCallTest(globalState);
        });

        cy.step("Void Payment without Confirmation", errorStack, () => {
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

        cy.then(() => {
          if (errorStack.length > 0) {
            reportErrors(errorStack);
          }
        });
      });
    }
  );

  context("Card - void payment in success state flow test", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Void Payment after Confirmation", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create Payment Intent", errorStack, () => {
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

      cy.step("Payment Methods Call", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment Intent", errorStack, () => {
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

      cy.step("Retrieve Payment after Confirmation", errorStack, () => {
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

      cy.step("Void Payment after Confirmation", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Void Payment after Confirmation");
          return;
        }
        const voidData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["VoidAfterConfirm"];

        cy.voidCallTest(fixtures.voidBody, voidData, globalState);
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });
});
