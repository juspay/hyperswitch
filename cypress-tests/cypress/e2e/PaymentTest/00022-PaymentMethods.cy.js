import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
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
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentMethodTest(globalState, req_data, res_data);
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
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentMethodTest(globalState, req_data, res_data);
    });

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntentOffSession"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("confirm-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "SaveCardUseNo3DSAutoCaptureOffSession"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("List PM for customer", () => {
      cy.listCustomerPMCallTest(globalState);
    });

    it("Set default payment method", () => {
      cy.setDefaultPaymentMethodTest(globalState);
    });
  });
});
