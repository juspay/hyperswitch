import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Crypto Payment", () => {
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

  context("Crypto Currency Payment flow", () => {
    it("Create Payment Intent -> Payment Methods Call Test -> Confirm Crypto Currency Payment -> Handle redirection -> Retrieve Payment Call Test", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "crypto_pm"
        ]["PaymentIntent"];

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

      cy.step("Confirm Crypto Currency Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Crypto Currency Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "crypto_pm"
        ]["CryptoCurrency"];

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
        cy.handleCryptoRedirection(
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
        cy.retrievePaymentCallTest({ globalState });
      });
    });
  });

  context("Crypto Currency manual capture flow", () => {
    it("Create Payment Intent -> Payment Methods Call Test -> Confirm Crypto Currency Payment -> Handle redirection -> Retrieve Payment Call Test", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "crypto_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
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

      cy.step("Confirm Crypto Currency Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Crypto Currency Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "crypto_pm"
        ]["CryptoCurrencyManualCapture"];

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
        cy.handleCryptoRedirection(
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
        cy.retrievePaymentCallTest({ globalState });
      });
    });
  });
});
