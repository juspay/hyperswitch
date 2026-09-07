import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card Redirect payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card Redirect - Full Refund flow test", () => {
    before(function () {
      if (
        utils.shouldIncludeConnector(
          globalState.get("connectorId"),
          utils.CONNECTOR_LISTS.INCLUDE.CARD_REDIRECT
        )
      ) {
        this.skip();
      }
    });

    it("create payment intent -> payment methods call -> confirm payment intent -> handle redirection -> retrieve payment -> refund payment -> sync refund payment", () => {
      let shouldContinue = true;

      cy.step("create payment intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_redirect_pm"
        ]["PaymentIntent"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("payment methods call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: payment methods call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("confirm payment intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: confirm payment intent");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_redirect_pm"
        ]["CardRedirect"];
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

      cy.step("handle redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: handle redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleCardRedirectRedirection(globalState, expected_redirection);
      });

      cy.step("retrieve payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: retrieve payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_redirect_pm"
        ]["CardRedirect"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("refund payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: refund payment");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_redirect_pm"
        ]["Refund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("sync refund payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: sync refund payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_redirect_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });

  context("Card Redirect - Partial Refund flow test", () => {
    before(function () {
      if (
        utils.shouldIncludeConnector(
          globalState.get("connectorId"),
          utils.CONNECTOR_LISTS.INCLUDE.CARD_REDIRECT
        )
      ) {
        this.skip();
      }
    });

    it("create payment intent -> payment methods call -> confirm payment intent -> handle redirection -> retrieve payment -> partial refund payment -> sync refund payment", () => {
      let shouldContinue = true;

      cy.step("create payment intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_redirect_pm"
        ]["PaymentIntent"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("payment methods call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: payment methods call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("confirm payment intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: confirm payment intent");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_redirect_pm"
        ]["CardRedirect"];
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

      cy.step("handle redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: handle redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleCardRedirectRedirection(globalState, expected_redirection);
      });

      cy.step("retrieve payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: retrieve payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_redirect_pm"
        ]["CardRedirect"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("partial refund payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: partial refund payment");
          return;
        }
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_redirect_pm"
        ]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(refundData)) {
          shouldContinue = false;
        }
      });

      cy.step("sync refund payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: sync refund payment");
          return;
        }
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_redirect_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
      });
    });
  });
});
