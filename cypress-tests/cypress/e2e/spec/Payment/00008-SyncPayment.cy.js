import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Sync payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should complete sync payment flow", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if(!utils.should_continue_further(confirmData)) return;

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });
});