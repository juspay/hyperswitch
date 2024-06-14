import confirmBody from "../../fixtures/confirm-body.json";
import createConfirmPaymentBody from "../../fixtures/create-confirm-body.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/utils";

let globalState;

describe("Card - NoThreeDS payment flow test", () => {
  let should_continue = true; // variable that will be used to skip tests if a previous test fails

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!should_continue) {
      this.skip();
    }
  });
  context("Card-NoThreeDS payment flow test Create and confirm", () => {
    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm No 3DS", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });
  });

  context("Card-NoThreeDS payment flow test Create+Confirm", () => {
    it("create+confirm-payment-call-test", () => {
      console.log("confirm -> " + globalState.get("connectorId"));
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createConfirmPaymentTest(
        createConfirmPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });
  });
});
