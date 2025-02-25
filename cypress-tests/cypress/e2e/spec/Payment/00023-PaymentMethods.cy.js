import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Payment Methods Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Create payment method for customer", () => {
    it("Create customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("Create Payment Method", () => {
      const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

      cy.createPaymentMethodTest(globalState, data);
    });

    it("List PM for customer", () => {
      cy.listCustomerPMCallTest(globalState);
    });
  });

  context("Set default payment method", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("List PM for customer", () => {
      cy.listCustomerPMCallTest(globalState);
    });

    it("Create Payment Method", () => {
      const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

      cy.createPaymentMethodTest(globalState, data);
    });

    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntentOffSession"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("confirm-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SaveCardUseNo3DSAutoCaptureOffSession"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("List PM for customer", () => {
      cy.listCustomerPMCallTest(globalState);
    });

    it("Set default payment method", () => {
      cy.setDefaultPaymentMethodTest(globalState);
    });
  });

  context("Delete payment method for customer", () => {
    it("Create customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("Create Payment Method", () => {
      const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
      cy.createPaymentMethodTest(globalState, data);
    });

    it("List PM for customer", () => {
      cy.listCustomerPMCallTest(globalState);
    });

    it("Delete Payment Method for a customer", () => {
      cy.deletePaymentMethodTest(globalState);
    });
  });
});
