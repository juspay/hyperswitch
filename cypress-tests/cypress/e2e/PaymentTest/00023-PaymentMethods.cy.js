import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import { validateConfig } from "../../utils/featureFlags";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;

describe("Payment Methods Tests", () => {
  let should_continue = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Create payment method for customer", () => {
    let should_continue = true;

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("Create customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("Create Payment Method", () => {
      let data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

      cy.createPaymentMethodTest(globalState, data);
    });

    it("List PM for customer", () => {
      cy.listCustomerPMCallTest(globalState);
    });
  });

  context("Set default payment method", () => {
    let should_continue = true;

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("List PM for customer", () => {
      cy.listCustomerPMCallTest(globalState);
    });

    it("Create Payment Method", () => {
      let data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];

      cy.createPaymentMethodTest(globalState, data);
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntentOffSession"
      ];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("confirm-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SaveCardUseNo3DSAutoCaptureOffSession"
      ];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("List PM for customer", () => {
      cy.listCustomerPMCallTest(globalState);
    });

    it("Set default payment method", () => {
      cy.setDefaultPaymentMethodTest(globalState);
    });
  });
});
