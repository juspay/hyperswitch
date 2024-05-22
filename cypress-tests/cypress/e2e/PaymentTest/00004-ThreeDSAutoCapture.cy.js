import confirmBody from "../../fixtures/confirm-body.json";
import createPaymentBody from "../../fixtures/create-payment-body.json";
import State from "../../utils/State";
import getConnectorDetails from "../ConnectorUtils/utils";
import * as utils from "../ConnectorUtils/utils";

let globalState;

describe("Card - ThreeDS payment flow test", () => {
  let should_continue = true; // variable that will be used to skip tests if a previous test fails

  beforeEach(function () { 
      if(!should_continue) {
          this.skip();
      }
  });

  before("seed global state", () => {

    cy.task('getGlobalState').then((state) => {
      globalState = new State(state);
      console.log("seeding globalState -> " + JSON.stringify(globalState));
      cy.task('cli_log', "SEEDING GLOBAL STATE -> " + JSON.stringify(globalState));
    })

  })

  afterEach("flush global state", () => {
    console.log("flushing globalState -> " + JSON.stringify(globalState));
    cy.task('setGlobalState', globalState.data);
    cy.task('cli_log', " FLUSHING GLOBAL STATE -> " + JSON.stringify(globalState));
  })


  it("create-payment-call-test", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    let req_data = data["Request"];
    let res_data = data["Response"];
    cy.createPaymentIntentTest(createPaymentBody, req_data, res_data, "three_ds", "automatic", globalState);
    if(should_continue) should_continue = utils.should_continue_further(res_data);
  });

  it("payment_methods-call-test", () => {
    cy.task('cli_log', "PM CALL ");
    cy.paymentMethodsCallTest(globalState);
  });

  it("Confirm 3DS", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSAutoCapture"];
    let req_data = data["Request"];
    let res_data = data["Response"];
    cy.task('cli_log', "GLOBAL STATE -> " + JSON.stringify(globalState.data));
    cy.confirmCallTest(confirmBody, req_data, res_data, true, globalState);
    if(should_continue) should_continue = utils.should_continue_further(res_data);
  });

  it("Handle redirection", () => {
    let expected_redirection = confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);
  })

});
