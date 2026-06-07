import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as RequestBodyUtils from "../../../utils/RequestBodyUtils";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

/**
 * Generates a random amount for connectors that require unique transaction amounts
 * to prevent duplicate transaction detection (e.g., Helcim)
 * @param {string} connectorId - The connector identifier
 * @returns {number|null} Random amount for Helcim, null for other connectors
 */
function getRandomAmountForConnector(connectorId) {
  if (connectorId === "helcim") {
    return RequestBodyUtils.generateRandomAmount(1000, 9000);
  }
  return null;
}

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
      let shouldContinue = true;
      // Generate random amount for Helcim to avoid duplicate transaction detection
      const randomAmount = getRandomAmountForConnector(globalState.get("connectorId"));

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        // Clone the body to avoid mutating the imported fixture
        const paymentBody = { ...fixtures.createPaymentBody };
        // Override amount for Helcim to use random amount
        if (randomAmount !== null) {
          paymentBody.amount = randomAmount;
        }

        cy.createPaymentIntentTest(
          paymentBody,
          data,
          "no_three_ds",
          "automatic",
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

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        // Clone the body to avoid mutating the imported fixture
        const confirmRequestBody = { ...fixtures.confirmBody };
        // Override amount for Helcim to use random amount (must match payment intent)
        if (randomAmount !== null) {
          confirmRequestBody.amount = randomAmount;
        }

        cy.confirmCallTest(
          confirmRequestBody,
          confirmData,
          true,
          globalState
        );

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Card-NoThreeDS payment flow test Create+Confirm", () => {
    it("Create and Confirm Payment -> Retrieve Payment", () => {
      let shouldContinue = true;
      // Generate random amount for Helcim to avoid duplicate transaction detection
      const randomAmount = getRandomAmountForConnector(globalState.get("connectorId"));

      cy.step("Create and Confirm Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        // Clone the body to avoid mutating the imported fixture
        const paymentBody = { ...fixtures.createConfirmPaymentBody };
        // Override amount for Helcim to use random amount
        if (randomAmount !== null) {
          paymentBody.amount = randomAmount;
        }

        cy.createConfirmPaymentTest(
          paymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Card-NoThreeDS payment with shipping cost", () => {
    it("Create Payment Intent with shipping cost -> Payment Methods Call -> Confirm Payment with shipping cost -> Retrieve Payment with shipping cost", () => {
      let shouldContinue = true;
      // Generate random amount for Helcim to avoid duplicate transaction detection
      const randomAmount = getRandomAmountForConnector(globalState.get("connectorId"));

      cy.step("Create Payment Intent with shipping cost", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentWithShippingCost"];

        // Clone the body to avoid mutating the imported fixture
        const paymentBody = { ...fixtures.createPaymentBody };
        // Override amount for Helcim to use random amount
        if (randomAmount !== null) {
          paymentBody.amount = randomAmount;
        }

        cy.createPaymentIntentTest(
          paymentBody,
          data,
          "no_three_ds",
          "automatic",
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

      cy.step("Confirm Payment with shipping cost", () => {
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

        // Clone the body to avoid mutating the imported fixture
        const confirmRequestBody = { ...fixtures.confirmBody };
        // Override amount for Helcim to use random amount (must match payment intent)
        if (randomAmount !== null) {
          confirmRequestBody.amount = randomAmount;
        }

        cy.confirmCallTest(
          confirmRequestBody,
          confirmData,
          true,
          globalState
        );

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment with shipping cost", () => {
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
    });
  });
});
