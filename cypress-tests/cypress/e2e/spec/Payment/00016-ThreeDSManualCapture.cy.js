import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - ThreeDS Manual payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should complete ThreeDS manual full capture with separate create and confirm", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "manual", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm 3DS payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Capture payment
    const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Retrieve payment after capture
    cy.retrievePaymentCallTest({ globalState, data: captureData });
  });

  it("should complete ThreeDS manual full capture with create+confirm", () => {
    // Create and confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];
    cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "three_ds", "manual", globalState);

    if (!utils.should_continue_further(confirmData)) return;

    // Handle redirection
    const expected_redirection = fixtures.createConfirmPaymentBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Capture payment
    const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Retrieve payment after capture
    cy.retrievePaymentCallTest({ globalState, data: captureData });
  });

  it("should complete ThreeDS manual partial capture with separate create and confirm", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "manual", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm 3DS payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Partial capture
    const partialCaptureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];
    cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState);

    // Retrieve payment after capture
    cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
  });

  it("should complete ThreeDS manual partial capture with create+confirm", () => {
    // Create and confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];
    cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "three_ds", "manual", globalState);

    if (!utils.should_continue_further(confirmData)) return;
    
    // Handle redirection
    const expected_redirection = fixtures.createConfirmPaymentBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Partial capture
    const partialCaptureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];
    cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState);

    // Retrieve payment after capture
    cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
  });
});