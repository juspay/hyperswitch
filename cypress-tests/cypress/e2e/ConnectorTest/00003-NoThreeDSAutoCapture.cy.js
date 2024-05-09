import confirmBody from "../../fixtures/confirm-body.json";
import createConfirmPaymentBody from "../../fixtures/create-confirm-body.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import State from "../../utils/State";
import getConnectorDetails from "../ConnectorUtils/utils";

let globalState;

describe("Card - NoThreeDS payment flow test", () => {

  before("seed global state", () => {

    cy.task('getGlobalState').then((state) => {
      globalState = new State(state);
      console.log("seeding globalState -> " + JSON.stringify(globalState));
    })
  })

  after("flush global state", () => {
    console.log("flushing globalState -> " + JSON.stringify(globalState));
    cy.task('setGlobalState', globalState.data);
  })

  context("Card-NoThreeDS payment flow test Create and confirm", () => {

    it("create-payment-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["PaymentIntent"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "no_three_ds", "automatic", globalState);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm No 3DS", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });

  });

  context("Card-NoThreeDS payment flow test Create+Confirm", () => {

    it("create+confirm-payment-call-test", () => {
      console.log("confirm -> " + globalState.get("connectorId"));
      let data = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createConfirmPaymentTest(createConfirmPaymentBody, req_data, res_data, "no_three_ds", "automatic", globalState);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });


  });
});