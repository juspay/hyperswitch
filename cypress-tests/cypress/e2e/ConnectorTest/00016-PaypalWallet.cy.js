import createPaymentBody from "../../fixtures/create-payment-body.json";
import confirmBody from "../../fixtures/confirm-body.json";
import getConnectorDetails from "../ConnectorUtils/utils";
import State from "../../utils/State";

let globalState;

describe("Wallet - paypal payment flow test", () => {

  before("seed global state", () => {

    cy.task('getGlobalState').then((state) => {
      globalState = new State(state);
      console.log("seeding globalState -> " + JSON.stringify(globalState));
    })
  })

  afterEach("flush global state", () => {
    console.log("flushing globalState -> " + JSON.stringify(globalState));
    cy.task('setGlobalState', globalState.data);
  })


context("Card-NoThreeDS payment flow test Create and confirm", () => {

    it("create-payment-call-test", () => {
      let det = getConnectorDetails(globalState.get("connectorId"))["wallet_pm"]["paypal"];
      cy.createPaymentIntentTest(createPaymentBody, det, "three_ds", "automatic", globalState);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm No 3DS", () => {
      let det = getConnectorDetails(globalState.get("connectorId"))["wallet_pm"]["paypal"];
      cy.paypalConfirmCallTest(confirmBody, det, globalState);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });
    
    it("Handle redirection", () => {
        let expected_redirection = confirmBody["return_url"];
        cy.handlePaypalRedirection(globalState, expected_redirection);
    })
    

  });

})