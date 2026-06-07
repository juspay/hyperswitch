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

describe("Card - NoThreeDS Manual payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Card - NoThreeDS Manual Full Capture payment flow test - Create and Confirm",
    () => {
      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture", () => {
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
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSManualCapture"];

          // Clone the body to avoid mutating the imported fixture
          const confirmRequestBody = { ...fixtures.confirmBody };
          // Apply the same random amount for consistency
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

        cy.step("Retrieve Payment after Confirmation", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Confirmation"
            );
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data: confirmData });

          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Capture Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Capture Payment");
            return;
          }
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];

          cy.captureCallTest(fixtures.captureBody, captureData, globalState);

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Capture", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment after Capture");
            return;
          }
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data: captureData });
        });
      });
    }
  );

  context(
    "Card - NoThreeDS Manual Full Capture payment flow test - Create+Confirm",
    () => {
      it("Create and Confirm Payment Intent -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture", () => {
        let shouldContinue = true;
        // Generate random amount for Helcim to avoid duplicate transaction detection
        const randomAmount = getRandomAmountForConnector(globalState.get("connectorId"));

        cy.step("Create and Confirm Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          // Clone the body to avoid mutating the imported fixture
          const createConfirmBody = { ...fixtures.createConfirmPaymentBody };
          // Override amount for Helcim to use random amount
          if (randomAmount !== null) {
            createConfirmBody.amount = randomAmount;
          }

          cy.createConfirmPaymentTest(
            createConfirmBody,
            data,
            "no_three_ds",
            "manual",
            globalState
          );

          if (!utils.should_continue_further(data)) {
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
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Capture Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Capture Payment");
            return;
          }
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];

          cy.captureCallTest(fixtures.captureBody, captureData, globalState);

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Capture", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment after Capture");
            return;
          }
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data: captureData });
        });
      });
    }
  );

  context(
    "Card - NoThreeDS Manual Partial Capture payment flow test - Create and Confirm",
    () => {
      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Partial Capture Payment -> Retrieve Payment after Partial Capture", () => {
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
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSManualCapture"];

          // Clone the body to avoid mutating the imported fixture
          const confirmRequestBody = { ...fixtures.confirmBody };
          // Apply the same random amount for consistency
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

        cy.step("Retrieve Payment after Confirmation", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Confirmation"
            );
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data: confirmData });

          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Capture Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Partial Capture Payment");
            return;
          }
          const partialCaptureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialCapture"];

          cy.captureCallTest(
            fixtures.captureBody,
            partialCaptureData,
            globalState
          );

          if (!utils.should_continue_further(partialCaptureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Partial Capture", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Partial Capture"
            );
            return;
          }
          const partialCaptureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialCapture"];

          cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
        });
      });
    }
  );

  context(
    "Card - NoThreeDS Manual Partial Capture payment flow test - Create+Confirm",
    () => {
      it("Create and Confirm Payment Intent -> Retrieve Payment after Confirmation -> Partial Capture Payment -> Retrieve Payment after Partial Capture", () => {
        let shouldContinue = true;
        // Generate random amount for Helcim to avoid duplicate transaction detection
        const randomAmount = getRandomAmountForConnector(globalState.get("connectorId"));

        cy.step("Create and Confirm Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          // Clone the body to avoid mutating the imported fixture
          const createConfirmBody = { ...fixtures.createConfirmPaymentBody };
          // Override amount for Helcim to use random amount
          if (randomAmount !== null) {
            createConfirmBody.amount = randomAmount;
          }

          cy.createConfirmPaymentTest(
            createConfirmBody,
            data,
            "no_three_ds",
            "manual",
            globalState
          );

          if (!utils.should_continue_further(data)) {
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
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Capture Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Partial Capture Payment");
            return;
          }
          const partialCaptureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialCapture"];

          cy.captureCallTest(
            fixtures.captureBody,
            partialCaptureData,
            globalState
          );

          if (!utils.should_continue_further(partialCaptureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Partial Capture", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Partial Capture"
            );
            return;
          }
          const partialCaptureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialCapture"];

          cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
        });
      });
    }
  );
});
