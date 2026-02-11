import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - NoThreeDS Manual payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should void payment in Requires_capture state", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "manual", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSManualCapture"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Void payment
    const voidData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["VoidAfterConfirm"];
    cy.voidCallTest(fixtures.voidBody, voidData, globalState);
  });

  it("should void payment in Requires_payment_method state", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "manual", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Void payment
    const voidData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Void"];
    cy.voidCallTest(fixtures.voidBody, voidData, globalState);
  });

  it("should void payment in success state", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "manual", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSManualCapture"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, false, globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Void payment
    const voidData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["VoidAfterConfirm"];
    cy.voidCallTest(fixtures.voidBody, voidData, globalState);
  });
});