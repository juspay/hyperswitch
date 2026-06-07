import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

/**
 * Generates a random amount for Helcim to avoid duplicate transaction blocking.
 * Helcim detects and rejects duplicate transactions by amount.
 */
function getRandomAmount() {
  return Math.floor(Math.random() * 8000) + 1000; // 1000 - 9000 cents
}

describe("Helcim - Refund E2E tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        // Skip if connector is not Helcim
        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.HELcim_REFUND
          )
        ) {
          skip = true;
          return;
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

  context("Helcim - No-3DS AutoCapture → Full Refund → Sync Refund", () => {
    it("Create Payment Intent → Payment Methods → Confirm Payment → Retrieve Payment → Refund Payment → Sync Refund Payment", () => {
      let shouldContinue = true;
      const randomAmount = getRandomAmount();

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        const paymentBody = { ...fixtures.createPaymentBody };
        paymentBody.amount = randomAmount;

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

        const confirmBody = { ...fixtures.confirmBody };
        confirmBody.amount = randomAmount;

        cy.confirmCallTest(confirmBody, confirmData, true, globalState);

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const retrieveData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data: retrieveData });

        if (!utils.should_continue_further(retrieveData)) {
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

        const refundBody = { ...fixtures.refundBody };
        refundBody.amount = randomAmount;

        cy.refundCallTest(refundBody, refundData, globalState);

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
    "Helcim - No-3DS AutoCapture → Partial Refund × 2 → Sync Refund",
    () => {
      it("Create Payment Intent → Payment Methods → Confirm Payment → Retrieve Payment → Partial Refund → 2nd Partial Refund → Sync Refund Payment", () => {
        let shouldContinue = true;
        const randomAmount = getRandomAmount();

        cy.step("Create Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          const paymentBody = { ...fixtures.createPaymentBody };
          paymentBody.amount = randomAmount;

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
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSAutoCapture"];

          const confirmBody = { ...fixtures.confirmBody };
          confirmBody.amount = randomAmount;

          cy.confirmCallTest(confirmBody, confirmData, true, globalState);

          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const retrieveData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data: retrieveData });

          if (!utils.should_continue_further(retrieveData)) {
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

          const refundBody = { ...fixtures.refundBody };
          refundBody.amount = Math.floor(randomAmount / 3);

          cy.refundCallTest(refundBody, partialRefundData, globalState);

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

          const refundBody = { ...fixtures.refundBody };
          refundBody.amount = Math.floor(randomAmount / 3);

          cy.refundCallTest(refundBody, partialRefundData, globalState);

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

  context(
    "Helcim - No-3DS ManualCapture → Capture → Full Refund → Sync Refund",
    () => {
      it("Create Payment Intent → Payment Methods → Confirm Payment → Retrieve Payment → Capture Payment → Retrieve Payment → Refund Payment → Sync Refund Payment", () => {
        let shouldContinue = true;
        const randomAmount = getRandomAmount();

        cy.step("Create Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          const paymentBody = { ...fixtures.createPaymentBody };
          paymentBody.amount = randomAmount;

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

        cy.step("Confirm Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSManualCapture"];

          const confirmBody = { ...fixtures.confirmBody };
          confirmBody.amount = randomAmount;

          cy.confirmCallTest(confirmBody, confirmData, true, globalState);

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
          const retrieveData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data: retrieveData });

          if (!utils.should_continue_further(retrieveData)) {
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

          const captureBody = { ...fixtures.captureBody };
          captureBody.amount_to_capture = randomAmount;

          cy.captureCallTest(captureBody, captureData, globalState);

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Capture", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment after Capture");
            return;
          }
          const retrieveData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data: retrieveData });

          if (!utils.should_continue_further(retrieveData)) {
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
          )["card_pm"]["manualPaymentRefund"];

          const refundBody = { ...fixtures.refundBody };
          refundBody.amount = randomAmount;

          cy.refundCallTest(refundBody, refundData, globalState);

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
    "Helcim - No-3DS ManualCapture → Partial Capture → Partial Refund → Sync Refund",
    () => {
      it("Create Payment Intent → Payment Methods → Confirm Payment → Retrieve Payment → Partial Capture → Retrieve Payment → Partial Refund → Sync Refund Payment", () => {
        let shouldContinue = true;
        const randomAmount = getRandomAmount();

        cy.step("Create Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          const paymentBody = { ...fixtures.createPaymentBody };
          paymentBody.amount = randomAmount;

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

        cy.step("Confirm Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSManualCapture"];

          const confirmBody = { ...fixtures.confirmBody };
          confirmBody.amount = randomAmount;

          cy.confirmCallTest(confirmBody, confirmData, true, globalState);

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
          const retrieveData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data: retrieveData });

          if (!utils.should_continue_further(retrieveData)) {
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

          const captureBody = { ...fixtures.captureBody };
          captureBody.amount_to_capture = Math.floor(randomAmount / 2);

          cy.captureCallTest(captureBody, partialCaptureData, globalState);

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
          const retrieveData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialCapture"];

          cy.retrievePaymentCallTest({ globalState, data: retrieveData });

          if (!utils.should_continue_further(retrieveData)) {
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

          const refundBody = { ...fixtures.refundBody };
          refundBody.amount = Math.floor(randomAmount / 4);

          cy.refundCallTest(refundBody, partialRefundData, globalState);

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
});
