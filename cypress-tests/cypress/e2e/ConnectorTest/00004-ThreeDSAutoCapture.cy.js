import createPaymentBody from "../../fixtures/create-payment-body.json";
import confirmBody from "../../fixtures/confirm-body.json";
import getConnectorDetails from "../ConnectorUtils/utils";
import State from "../../utils/State";

let globalState;

describe("Card - ThreeDS payment flow test", () => {

  before("seed global state", () => {

    cy.task('getGlobalState').then((state) => {
      // visit non same-origin url https://www.cypress-dx.com
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
    let det = getConnectorDetails(globalState.get("connectorId"))["No3DS"];
    cy.createPaymentIntentTest(createPaymentBody, det.currency, "three_ds", "automatic", globalState);
  });

  it("payment_methods-call-test", () => {
    cy.task('cli_log', "PM CALL ");
    cy.paymentMethodsCallTest(globalState);
  });

  it("Confirm 3DS", () => {
    let det = getConnectorDetails(globalState.get("connectorId"))["3DS"];
    cy.task('cli_log', "GLOBAL STATE -> " + JSON.stringify(globalState.data));
    cy.confirmCallTest(confirmBody, det, true, globalState);
  });
});