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
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Payment Methods Call Test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm Crypto Currency Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "crypto_pm"
      ]["CryptoCurrency"];

      cy.confirmRewardCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handleCryptoRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("Retrieve Payment Call Test", () => {
      cy.retrievePaymentCallTest(globalState);
    });
  });

  context("Crypto Currency manual capture flow", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Payment Methods Call Test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm Crypto Currency Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "crypto_pm"
      ]["CryptoCurrencyManualCapture"];

      cy.confirmRewardCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handleCryptoRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("Retrieve Payment Call Test", () => {
      cy.retrievePaymentCallTest(globalState);
    });
  });
});
