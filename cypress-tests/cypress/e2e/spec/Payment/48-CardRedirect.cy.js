import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Card Redirect tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.CARD_REDIRECT
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

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Benefit Create and Confirm flow test", () => {
    let shouldContinue = true;

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_redirect_pm"
      ]["PaymentIntent"]("Benefit");
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
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-card-redirect-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_redirect_pm"
      ]["Benefit"];
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("handle-card-redirect-redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      const nextActionUrl = globalState.get("nextActionUrl");

      expect(
        nextActionUrl,
        "nextActionUrl should be defined before handling card redirect redirection"
      ).to.be.a("string");

      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_redirect_pm"
      ]["Benefit"];
      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  context("Knet Create and Confirm flow test", () => {
    let shouldContinue = true;

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_redirect_pm"
      ]["PaymentIntent"]("Knet");
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
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-card-redirect-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_redirect_pm"
      ]["Knet"];
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("handle-card-redirect-redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      const nextActionUrl = globalState.get("nextActionUrl");

      expect(
        nextActionUrl,
        "nextActionUrl should be defined before handling card redirect redirection"
      ).to.be.a("string");

      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_redirect_pm"
      ]["Knet"];
      cy.retrievePaymentCallTest({ globalState, data });
    });
  });

  context("MomoAtm Create and Confirm flow test", () => {
    let shouldContinue = true;

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_redirect_pm"
      ]["PaymentIntent"]("MomoAtm");
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
      cy.paymentMethodsCallTest(globalState);
    });

    it("confirm-card-redirect-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_redirect_pm"
      ]["MomoAtm"];
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("handle-card-redirect-redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      const nextActionUrl = globalState.get("nextActionUrl");

      expect(
        nextActionUrl,
        "nextActionUrl should be defined before handling card redirect redirection"
      ).to.be.a("string");

      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });

    it("retrieve-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_redirect_pm"
      ]["MomoAtm"];
      cy.retrievePaymentCallTest({ globalState, data });
    });
  });
});
