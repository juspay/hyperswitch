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

describe("Card - Refund flow - No 3DS", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card - Full Refund flow test for No-3DS", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Refund Payment -> Sync Refund Payment", () => {
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

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

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
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund Payment");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Partial Refund flow test for No-3DS", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment", () => {
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

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

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
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Partial Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Partial Refund Payment");
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Partial Refund Payment - 2nd Attempt", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Partial Refund Payment - 2nd Attempt"
          );
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context(
    "Fully Refund Card-NoThreeDS payment flow test Create+Confirm",
    () => {
      it("Create and Confirm Payment -> Retrieve Payment after Confirmation -> Refund Payment -> Sync Refund Payment", () => {
        let shouldContinue = true;
        // Generate random amount for Helcim to avoid duplicate transaction detection
        const randomAmount = getRandomAmountForConnector(globalState.get("connectorId"));

        cy.step("Create and Confirm Payment", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

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
            "automatic",
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
          ]["No3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data });
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Refund Payment");
            return;
          }
          const refundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Refund"];
          cy.refundCallTest(fixtures.refundBody, refundData, globalState);
          if (!utils.should_continue_further(refundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Sync Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Sync Refund Payment");
            return;
          }
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];
          cy.syncRefundCallTest(syncRefundData, globalState);
        });
      });
    }
  );

  context(
    "Partially Refund Card-NoThreeDS payment flow test Create+Confirm",
    () => {
      it("Create and Confirm Payment -> Retrieve Payment after Confirmation -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment", () => {
        let shouldContinue = true;
        // Generate random amount for Helcim to avoid duplicate transaction detection
        const randomAmount = getRandomAmountForConnector(globalState.get("connectorId"));

        cy.step("Create and Confirm Payment", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

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
            "automatic",
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
          ]["No3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data });
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Partial Refund Payment");
            return;
          }
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialRefund"];
          cy.refundCallTest(
            fixtures.refundBody,
            partialRefundData,
            globalState
          );
          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Refund Payment - 2nd Attempt", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Partial Refund Payment - 2nd Attempt"
            );
            return;
          }
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialRefund"];
          cy.refundCallTest(
            fixtures.refundBody,
            partialRefundData,
            globalState
          );
          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Sync Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Sync Refund Payment");
            return;
          }
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];
          const newData = {
            ...syncRefundData,
            Response: syncRefundData.ResponseCustom || syncRefundData.Response,
          };
          cy.refundCallTest(fixtures.refundBody, newData, globalState);
        });
      });
    }
  );

  context("Card - Full Refund for fully captured No-3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture -> Refund Payment -> Sync Refund Payment", () => {
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
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

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
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
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
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
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
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.retrievePaymentCallTest({ globalState, data: captureData });
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund Payment");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["manualPaymentRefund"];
        const newRefundData = {
          ...refundData,
          Response: refundData.ResponseCustom || refundData.Response,
        };
        cy.refundCallTest(fixtures.refundBody, newRefundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Partial Refund for fully captured No-3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment -> List Refunds", () => {
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
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

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
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
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
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
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
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.retrievePaymentCallTest({ globalState, data: captureData });
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      cy.step("Partial Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Partial Refund Payment");
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        const newPartialRefundData = {
          ...partialRefundData,
          Response:
            partialRefundData.ResponseCustom || partialRefundData.Response,
        };
        cy.refundCallTest(
          fixtures.refundBody,
          newPartialRefundData,
          globalState
        );
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Partial Refund Payment - 2nd Attempt", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Partial Refund Payment - 2nd Attempt"
          );
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        const newPartialRefundData = {
          ...partialRefundData,
          Response:
            partialRefundData.ResponseCustom || partialRefundData.Response,
        };
        cy.refundCallTest(
          fixtures.refundBody,
          newPartialRefundData,
          globalState
        );
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
        if (!utils.should_continue_further(syncRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("List Refunds", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Refunds");
          return;
        }
        cy.listRefundCallTest(fixtures.listRefundCall, globalState);
      });
    });
  });

  context("Card - Full Refund for partially captured No-3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Partial Capture Payment -> Retrieve Payment after Partial Capture -> Refund Payment -> Sync Refund Payment", () => {
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
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

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
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
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
        if (!utils.should_continue_further(partialCaptureData)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund Payment");
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        const newPartialRefundData = {
          ...partialRefundData,
          Response:
            partialRefundData.ResponseCustom || partialRefundData.Response,
        };
        cy.refundCallTest(
          fixtures.refundBody,
          newPartialRefundData,
          globalState
        );
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Partial Refund for partially captured No-3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Partial Capture Payment -> Retrieve Payment after Partial Capture -> Refund Payment -> Sync Refund Payment", () => {
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
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

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
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
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
        if (!utils.should_continue_further(partialCaptureData)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund Payment");
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        const newPartialRefundData = {
          ...partialRefundData,
          Response:
            partialRefundData.ResponseCustom || partialRefundData.Response,
        };
        cy.refundCallTest(
          fixtures.refundBody,
          newPartialRefundData,
          globalState
        );
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });
});

describe("Card - Refund flow - 3DS", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card - Full Refund flow test for 3DS", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment after Confirmation -> Refund Payment -> Sync Refund Payment", () => {
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
          "three_ds",
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

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];

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

      cy.step("Handle Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
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
        ]["3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund Payment");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Partial Refund flow test for 3DS", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment after Confirmation -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment", () => {
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
          "three_ds",
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

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];

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

      cy.step("Handle Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
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
        ]["3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Partial Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Partial Refund Payment");
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Partial Refund Payment - 2nd Attempt", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Partial Refund Payment - 2nd Attempt"
          );
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Fully Refund Card-ThreeDS payment flow test Create+Confirm", () => {
    it("Create and Confirm Payment -> Handle Redirection -> Retrieve Payment after Confirmation -> Refund Payment -> Sync Refund Payment", () => {
      let shouldContinue = true;
      // Generate random amount for Helcim to avoid duplicate transaction detection
      const randomAmount = getRandomAmountForConnector(globalState.get("connectorId"));

      cy.step("Create and Confirm Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];

        // Clone the body to avoid mutating the imported fixture
        const createConfirmBody = { ...fixtures.createConfirmPaymentBody };
        // Override amount for Helcim to use random amount
        if (randomAmount !== null) {
          createConfirmBody.amount = randomAmount;
        }

        cy.createConfirmPaymentTest(
          createConfirmBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
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
        ]["3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data });
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund Payment");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context(
    "Partially Refund Card-ThreeDS payment flow test Create+Confirm",
    () => {
      it("Create and Confirm Payment -> Handle Redirection -> Retrieve Payment after Confirmation -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment", () => {
        let shouldContinue = true;
        // Generate random amount for Helcim to avoid duplicate transaction detection
        const randomAmount = getRandomAmountForConnector(globalState.get("connectorId"));

        cy.step("Create and Confirm Payment", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSAutoCapture"];

          // Clone the body to avoid mutating the imported fixture
          const createConfirmBody = { ...fixtures.createConfirmPaymentBody };
          // Override amount for Helcim to use random amount
          if (randomAmount !== null) {
            createConfirmBody.amount = randomAmount;
          }

          cy.createConfirmPaymentTest(
            createConfirmBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle Redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle Redirection");
            return;
          }
          const expected_redirection = fixtures.confirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
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
          ]["3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data });
          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Partial Refund Payment");
            return;
          }
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialRefund"];
          cy.refundCallTest(
            fixtures.refundBody,
            partialRefundData,
            globalState
          );
          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Refund Payment - 2nd Attempt", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Partial Refund Payment - 2nd Attempt"
            );
            return;
          }
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialRefund"];
          cy.refundCallTest(
            fixtures.refundBody,
            partialRefundData,
            globalState
          );
          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Sync Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Sync Refund Payment");
            return;
          }
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];
          cy.syncRefundCallTest(syncRefundData, globalState);
        });
      });
    }
  );

  context("Card - Full Refund for fully captured 3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture -> Refund Payment -> Sync Refund Payment", () => {
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
          "three_ds",
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
        ]["3DSManualCapture"];

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

      cy.step("Handle Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
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
        ]["3DSManualCapture"];
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
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
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
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.retrievePaymentCallTest({ globalState, data: captureData });
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund Payment");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["manualPaymentRefund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Partial Refund for fully captured 3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment", () => {
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
          "three_ds",
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
        ]["3DSManualCapture"];

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

      cy.step("Handle Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
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
        ]["3DSManualCapture"];
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
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
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
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.retrievePaymentCallTest({ globalState, data: captureData });
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      cy.step("Partial Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Partial Refund Payment");
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Partial Refund Payment - 2nd Attempt", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Partial Refund Payment - 2nd Attempt"
          );
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Full Refund for partially captured 3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment after Confirmation -> Partial Capture Payment -> Retrieve Payment after Partial Capture -> Refund Payment -> Sync Refund Payment", () => {
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
          "three_ds",
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
        ]["3DSManualCapture"];

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

      cy.step("Handle Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
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
        ]["3DSManualCapture"];
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
        if (!utils.should_continue_further(partialCaptureData)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund Payment");
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Partial Refund for partially captured 3DS payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment after Confirmation -> Partial Capture Payment -> Retrieve Payment after Partial Capture -> Partial Refund Payment -> Sync Refund Payment", () => {
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
          "three_ds",
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
        ]["3DSManualCapture"];

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

      cy.step("Handle Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
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
        ]["3DSManualCapture"];
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
        if (!utils.should_continue_further(partialCaptureData)) {
          shouldContinue = false;
        }
      });

      cy.step("Partial Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Partial Refund Payment");
          return;
        }
        const partialRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["manualPaymentPartialRefund"];
        const newPartialRefundData = {
          ...partialRefundData,
          Request: { amount: partialRefundData.Request.amount / 2 },
        };
        cy.refundCallTest(
          fixtures.refundBody,
          newPartialRefundData,
          globalState
        );
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });
});
