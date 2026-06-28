import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import { amazonPayPaymentMethodsEnabled } from "../../configs/Payment/Amazonpay";

let globalState;

describe("Amazon Pay - Refund flow", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Amazon Pay - Setup and Full Refund flow", () => {
    it("merchant-create-call-test", () => {
      cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
    });

    it("api-key-create-call-test", () => {
      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
    });

    it("customer-create-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("create-connector-call-test", () => {
      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        amazonPayPaymentMethodsEnabled,
        globalState
      );
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaymentIntent"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-payment-call-test", () => {
      let shouldContinue = true;
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["No3DSAutoCapture"];
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        confirmData,
        true,
        globalState
      );
      if (!utils.should_continue_further(confirmData)) {
        shouldContinue = false;
      }
    });

    it("handle-wallet-redirection-test", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      const nextActionUrl = globalState.get("nextActionUrl");

      if (!nextActionUrl) {
        cy.task("cli_log", "No nextActionUrl - skipping wallet redirection");
        return;
      }

      cy.handleWalletRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("retrieve-payment-after-confirmation-test", () => {
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["No3DSAutoCapture"];
      cy.retrievePaymentCallTest({ globalState, data: confirmData });
    });

    it("refund-payment-call-test", () => {
      const refundData = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["Refund"];
      cy.refundCallTest(fixtures.refundBody, refundData, globalState);
    });

    it("sync-refund-call-test", () => {
      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["SyncRefund"];
      cy.syncRefundCallTest(syncRefundData, globalState);
    });
  });

  context("Amazon Pay - Partial Refund flow", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["PaymentIntent"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-payment-call-test", () => {
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["No3DSAutoCapture"];
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        confirmData,
        true,
        globalState
      );
    });

    it("handle-wallet-redirection-test", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      const nextActionUrl = globalState.get("nextActionUrl");

      if (!nextActionUrl) {
        cy.task("cli_log", "No nextActionUrl - skipping wallet redirection");
        return;
      }

      cy.handleWalletRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("retrieve-payment-after-confirmation-test", () => {
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["No3DSAutoCapture"];
      cy.retrievePaymentCallTest({ globalState, data: confirmData });
    });

    it("partial-refund-payment-call-test", () => {
      const partialRefundData = getConnectorDetails(
        globalState.get("connectorId")
      )["wallet_pm"]["PartialRefund"];
      cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
    });

    it("partial-refund-payment-2nd-attempt-test", () => {
      const partialRefundData = getConnectorDetails(
        globalState.get("connectorId")
      )["wallet_pm"]["PartialRefund"];
      cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
    });

    it("sync-refund-call-test", () => {
      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["SyncRefund"];
      cy.syncRefundCallTest(syncRefundData, globalState);
    });
  });
});
