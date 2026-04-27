import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

const KLARNA_REDIRECT_CONNECTORS = [];

describe("PayLater tests", () => {
  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Klarna PayLater - Auto Capture flow test", function () {
    let shouldContinue = true;

    before(function () {
      cy.task("getGlobalState")
        .then((state) => {
          globalState = new State(state);
          if (
            utils.shouldIncludeConnector(
              globalState.get("connectorId"),
              utils.CONNECTOR_LISTS.INCLUDE.PAY_LATER
            )
          ) {
            shouldContinue = false;
          }
        })
        .then(() => {
          if (!shouldContinue) {
            this.skip();
          }
        });
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "pay_later_pm"
      ]["AutoCapture"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      if (!shouldContinue) return;
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-paylater-redirect-test", () => {
      if (!shouldContinue) return;
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "pay_later_pm"
      ]["Klarna"];
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("handle-paylater-redirect-test", () => {
      if (!shouldContinue) return;
      const connectorId = globalState.get("connectorId");
      if (!KLARNA_REDIRECT_CONNECTORS.includes(connectorId)) return;
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("verify-paylater-status-test", () => {
      if (!shouldContinue) return;
      const connectorId = globalState.get("connectorId");
      if (KLARNA_REDIRECT_CONNECTORS.includes(connectorId)) {
        cy.retrievePaymentCallTest(globalState);
      } else {
        cy.log(
          "Klarna redirect not supported in test environment - verifying payment state"
        );
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.retrievePaymentCallTest({ globalState, data });
      }
    });
  });

  context("Klarna PayLater - Manual Capture flow test", function () {
    let shouldContinue = true;

    before(function () {
      cy.task("getGlobalState")
        .then((state) => {
          globalState = new State(state);
          if (
            utils.shouldIncludeConnector(
              globalState.get("connectorId"),
              utils.CONNECTOR_LISTS.INCLUDE.PAY_LATER
            )
          ) {
            shouldContinue = false;
          }
        })
        .then(() => {
          if (!shouldContinue) {
            this.skip();
          }
        });
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "pay_later_pm"
      ]["ManualCapture"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "manual",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      if (!shouldContinue) return;
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-paylater-redirect-test", () => {
      if (!shouldContinue) return;
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "pay_later_pm"
      ]["Klarna"];
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("handle-paylater-redirect-test", () => {
      if (!shouldContinue) return;
      const connectorId = globalState.get("connectorId");
      if (!KLARNA_REDIRECT_CONNECTORS.includes(connectorId)) return;
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("capture-paylater-call-test", () => {
      if (!shouldContinue) return;
      const connectorId = globalState.get("connectorId");
      if (!KLARNA_REDIRECT_CONNECTORS.includes(connectorId)) return;
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "pay_later_pm"
      ]["Capture"];
      cy.captureCallTest(fixtures.captureBody, data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("refund-paylater-call-test", () => {
      if (!shouldContinue) return;
      const connectorId = globalState.get("connectorId");
      if (!KLARNA_REDIRECT_CONNECTORS.includes(connectorId)) return;
      const refundData = getConnectorDetails(globalState.get("connectorId"))[
        "pay_later_pm"
      ]["Refund"];
      const newRefundData = {
        ...refundData,
        Response: refundData.ResponseCustom || refundData.Response,
      };
      cy.refundCallTest(fixtures.refundBody, newRefundData, globalState);
    });

    it("verify-paylater-status-test", () => {
      if (!shouldContinue) return;
      const connectorId = globalState.get("connectorId");
      if (KLARNA_REDIRECT_CONNECTORS.includes(connectorId)) {
        cy.retrievePaymentCallTest(globalState);
      } else {
        cy.log(
          "Klarna redirect not supported in test environment - verifying payment state"
        );
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pay_later_pm"
        ]["Klarna"];
        cy.retrievePaymentCallTest({ globalState, data });
      }
    });
  });
});
