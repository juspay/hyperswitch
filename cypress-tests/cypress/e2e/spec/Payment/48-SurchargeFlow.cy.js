import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Surcharge Flow Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card - Payment with Surcharge - Happy Path", () => {
    it("Create Payment Intent with surcharge_details - Confirm - Verify net_amount includes surcharge", () => {
      let shouldContinue = true;

      cy.step("Check if connector supports surcharge", () => {
        const connectorId = globalState.get("connectorId");
        if (!utils.CONNECTOR_LISTS.INCLUDE.SURCHARGE.includes(connectorId)) {
          cy.task("cli_log", `Skipping surcharge flow: connector ${connectorId} not in SURCHARGE list`);
          shouldContinue = false;
        }
      });

      cy.step("Create Payment Intent with surcharge", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Payment Intent with surcharge");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentWithSurcharge"];
        cy.createPaymentIntentTest(fixtures.createPaymentBody, data, "no_three_ds", "automatic", globalState);
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

      cy.step("Confirm Payment with surcharge", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with surcharge");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ConfirmPaymentWithSurcharge"];
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment with surcharge", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment with surcharge");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ConfirmPaymentWithSurcharge"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Card - Payment with Surcharge - Manual Capture", () => {
    it("Create Payment with surcharge - Capture - Verify net_amount", () => {
      let shouldContinue = true;

      cy.step("Check if connector supports surcharge", () => {
        const connectorId = globalState.get("connectorId");
        if (!utils.CONNECTOR_LISTS.INCLUDE.SURCHARGE.includes(connectorId)) {
          cy.task("cli_log", `Skipping surcharge flow: connector ${connectorId} not in SURCHARGE list`);
          shouldContinue = false;
        }
      });

      cy.step("Create Payment Intent with surcharge (manual capture)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Payment Intent");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentWithSurcharge"];
        cy.createPaymentIntentTest(fixtures.createPaymentBody, data, "no_three_ds", "manual", globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSManualCapture"];
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Capture Payment with surcharge", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Capture Payment");
          return;
        }
        const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
        cy.captureCallTest(captureData, globalState);
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment to verify surcharge", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const retrieveData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ConfirmPaymentWithSurcharge"];
        cy.retrievePaymentCallTest({ globalState, data: retrieveData });
      });
    });
  });

  context("Card - Payment with Surcharge - Create and Confirm", () => {
    it("Create and Confirm Payment with surcharge_details - Retrieve - Verify net_amount", () => {
      let shouldContinue = true;

      cy.step("Check if connector supports surcharge", () => {
        const connectorId = globalState.get("connectorId");
        if (!utils.CONNECTOR_LISTS.INCLUDE.SURCHARGE.includes(connectorId)) {
          cy.task("cli_log", `Skipping surcharge flow: connector ${connectorId} not in SURCHARGE list`);
          shouldContinue = false;
        }
      });

      cy.step("Create and Confirm Payment with surcharge", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create and Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ConfirmPaymentWithSurcharge"];
        cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, data, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment to verify surcharge", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const retrieveData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ConfirmPaymentWithSurcharge"];
        cy.retrievePaymentCallTest({ globalState, data: retrieveData });
      });
    });
  });

  context("Card - Payment with Surcharge - Sync Payment", () => {
    it("Create Payment with surcharge - Sync - Verify surcharge details persist", () => {
      let shouldContinue = true;

      cy.step("Check if connector supports surcharge", () => {
        const connectorId = globalState.get("connectorId");
        if (!utils.CONNECTOR_LISTS.INCLUDE.SURCHARGE.includes(connectorId)) {
          cy.task("cli_log", `Skipping surcharge flow: connector ${connectorId} not in SURCHARGE list`);
          shouldContinue = false;
        }
      });

      cy.step("Create Payment Intent with surcharge", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Payment Intent");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentWithSurcharge"];
        cy.createPaymentIntentTest(fixtures.createPaymentBody, data, "no_three_ds", "automatic", globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ConfirmPaymentWithSurcharge"];
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Payment to verify surcharge details", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Payment");
          return;
        }
        const syncData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncPayment"];
        cy.syncPaymentCallTest(syncData, globalState);
      });
    });
  });

  context("Card - Payment with Surcharge - Full Refund", () => {
    it("Create Payment with surcharge - Confirm - Full Refund - Verify refund amount", () => {
      let shouldContinue = true;

      cy.step("Check if connector supports surcharge", () => {
        const connectorId = globalState.get("connectorId");
        if (!utils.CONNECTOR_LISTS.INCLUDE.SURCHARGE.includes(connectorId)) {
          cy.task("cli_log", `Skipping surcharge flow: connector ${connectorId} not in SURCHARGE list`);
          shouldContinue = false;
        }
      });

      cy.step("Create Payment Intent with surcharge", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Payment Intent");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentWithSurcharge"];
        cy.createPaymentIntentTest(fixtures.createPaymentBody, data, "no_three_ds", "automatic", globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ConfirmPaymentWithSurcharge"];
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Full Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Full Refund Payment");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
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
        const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card - Payment with Surcharge - Partial Refund", () => {
    it("Create Payment with surcharge - Confirm - Partial Refund - Verify refund amount", () => {
      let shouldContinue = true;

      cy.step("Check if connector supports surcharge", () => {
        const connectorId = globalState.get("connectorId");
        if (!utils.CONNECTOR_LISTS.INCLUDE.SURCHARGE.includes(connectorId)) {
          cy.task("cli_log", `Skipping surcharge flow: connector ${connectorId} not in SURCHARGE list`);
          shouldContinue = false;
        }
      });

      cy.step("Create Payment Intent with surcharge", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create Payment Intent");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentWithSurcharge"];
        cy.createPaymentIntentTest(fixtures.createPaymentBody, data, "no_three_ds", "automatic", globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ConfirmPaymentWithSurcharge"];
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Partial Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Partial Refund Payment");
          return;
        }
        const partialRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
        cy.refundCallTest(fixtures.partiallyRefundBody, partialRefundData, globalState);
        if (!utils.should_continue_further(partialRefundData)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync Refund Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync Refund Payment");
          return;
        }
        const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });
});
