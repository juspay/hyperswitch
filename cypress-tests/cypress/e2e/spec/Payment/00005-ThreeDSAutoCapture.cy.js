import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - ThreeDS payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should complete 3DS payment flow", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      createPaymentData,
      "three_ds",
      "automatic",
      globalState
    );

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm with 3DS
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSAutoCapture"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);
  });
});