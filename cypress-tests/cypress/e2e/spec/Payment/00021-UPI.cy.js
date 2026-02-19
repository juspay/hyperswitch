import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("UPI Payments - Hyperswitch", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should complete UPI Collect payment and refund", () => {
    const createPaymentData = getConnectorDetails(
      globalState.get("connectorId")
    )["upi_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      createPaymentData,
      "three_ds",
      "automatic",
      globalState
    );

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "upi_pm"
    ]["UpiCollect"];
    cy.confirmUpiCall(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");
    cy.handleUpiRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    );

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    const refundData = getConnectorDetails(globalState.get("connectorId"))[
      "upi_pm"
    ]["Refund"];
    cy.refundCallTest(fixtures.refundBody, refundData, globalState);
  });

  // Skipping UPI Intent intentionally as connector is throwing 5xx during redirection
  it.skip("should complete UPI Intent payment", () => {
    const createPaymentData = getConnectorDetails(
      globalState.get("connectorId")
    )["upi_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      createPaymentData,
      "three_ds",
      "automatic",
      globalState
    );

    cy.paymentMethodsCallTest(globalState);

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "upi_pm"
    ]["UpiIntent"];
    cy.confirmUpiCall(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");
    cy.handleUpiRedirection(
      globalState,
      payment_method_type,
      expected_redirection
    );

    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });
});

// TODO: This test is incomplete. Above has to be replicated here with changes to support SCL
describe.skip("UPI Payments -- Hyperswitch Stripe Compatibility Layer", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });
});