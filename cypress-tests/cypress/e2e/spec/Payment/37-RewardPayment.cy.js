import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Reward Payment - Cashtocode", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Evoucher payment method flow", () => {
    it("Create Payment Intent for Evoucher -> Payment Methods Call Test -> Confirm Evoucher Payment -> Handle redirection -> Retrieve Payment Call Test", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent for Evoucher", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "reward_pm"
        ]["PaymentIntentUSD"];

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

      cy.step("Payment Methods Call Test", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call Test");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Evoucher Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Evoucher Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "reward_pm"
        ]["Evoucher"];

        cy.confirmRewardCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleRewardRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment Call Test", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment Call Test");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "reward_pm"
        ]["Evoucher"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Classic payment method flow", () => {
    it("Create Payment Intent for Classic -> Payment Methods Call Test -> Confirm Classic Payment -> Handle Redirection for Classic -> Retrieve Payment Call Test", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent for Classic", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "reward_pm"
        ]["PaymentIntentEUR"];

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

      cy.step("Payment Methods Call Test", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call Test");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Classic Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Classic Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "reward_pm"
        ]["Classic"];

        cy.confirmRewardCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Redirection for Classic", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Redirection for Classic");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleRewardRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve Payment Call Test", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment Call Test");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "reward_pm"
        ]["Classic"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });
});
