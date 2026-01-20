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
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent for Evoucher", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Payment Methods Call Test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm Evoucher Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "reward_pm"
      ]["Evoucher"];

      cy.confirmRewardCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handleRewardRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("Retrieve Payment Call Test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "reward_pm"
      ]["Evoucher"];

      cy.retrievePaymentCallTest(globalState, data);
    });
  });

  context("Classic payment method flow", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent for Classic", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Payment Methods Call Test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm Classic Payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "reward_pm"
      ]["Classic"];

      cy.confirmRewardCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle Redirection for Classic", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handleRewardRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("Retrieve Payment Call Test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "reward_pm"
      ]["Classic"];

      cy.retrievePaymentCallTest(globalState, data);
    });
  });
});
