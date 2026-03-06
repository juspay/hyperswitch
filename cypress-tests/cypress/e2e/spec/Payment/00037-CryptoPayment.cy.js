import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import step from "../../../utils/customStep";

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
    it("Create Payment Intent -> Payment Methods Call -> Confirm Crypto Currency Payment -> Handle Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
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

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Crypto Currency Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "crypto_pm"
        ]["CryptoCurrency"];
        cy.confirmRewardCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Handle Redirection", shouldContinue, () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleCryptoRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      step("Retrieve Payment", shouldContinue, () => {
        cy.retrievePaymentCallTest({ globalState });
      });
    });
  });

  context("Crypto Currency manual capture flow", () => {
    it("Create Payment Intent (Manual) -> Payment Methods Call -> Confirm Crypto Currency Payment (Manual Capture) -> Handle Redirection -> Retrieve Payment", () => {
      let shouldContinue = true;

      step("Create Payment Intent (Manual)", shouldContinue, () => {
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

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step(
        "Confirm Crypto Currency Payment (Manual Capture)",
        shouldContinue,
        () => {
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["crypto_pm"]["CryptoCurrencyManualCapture"];
          cy.confirmRewardCallTest(
            fixtures.confirmBody,
            confirmData,
            true,
            globalState
          );
          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        }
      );

      step("Handle Redirection", shouldContinue, () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleCryptoRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      step("Retrieve Payment", shouldContinue, () => {
        cy.retrievePaymentCallTest({ globalState });
      });
    });
  });
});
