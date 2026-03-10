import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import reportErrors from "../../../utils/reportErrors";

let globalState;

describe("Card - NoThreeDS payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card-NoThreeDS payment flow test Create and confirm", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment -> Retrieve Payment", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.stepTest("Create Payment Intent", errorStack, () => {
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.stepTest("Payment Methods Call", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.stepTest("Confirm Payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

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

      cy.stepTest("Retrieve Payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("Card-NoThreeDS payment flow test Create+Confirm", () => {
    it("Create and Confirm Payment -> Retrieve Payment", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.stepTest("Create and Confirm Payment", errorStack, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.stepTest("Retrieve Payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("Card-NoThreeDS payment with shipping cost", () => {
    it("Create Payment Intent with shipping cost -> Payment Methods Call -> Confirm Payment with shipping cost -> Retrieve Payment with shipping cost", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.stepTest("Create Payment Intent with shipping cost", errorStack, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentWithShippingCost"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
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
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.stepTest("Payment Methods Call", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.stepTest("Confirm Payment with shipping cost", errorStack, () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment with shipping cost"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentConfirmWithShippingCost"];

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

      cy.stepTest("Retrieve Payment with shipping cost", errorStack, () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment with shipping cost"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentConfirmWithShippingCost"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });
});
