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

  context("Card - NoThreeDS Manual Full Capture payment flow test", () => {
    it("should complete payment with separate create and confirm", () => {
      // Create payment intent
      const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
      cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "manual", globalState);

      // List payment methods
      cy.paymentMethodsCallTest(globalState);

      // Confirm payment
      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSManualCapture"];
      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

      if(!utils.should_continue_further(confirmData)) return;

      // Retrieve payment
      cy.retrievePaymentCallTest({ globalState, data: confirmData });

      // Capture payment
      const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
      cy.captureCallTest(fixtures.captureBody, captureData, globalState);

      // Retrieve payment after capture
      cy.retrievePaymentCallTest({ globalState, data: captureData });
    });

    it("should complete payment with create+confirm", () => {
      // Create and confirm payment
      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSManualCapture"];
      cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "no_three_ds", "manual", globalState);

      if(!utils.should_continue_further(confirmData)) return;

      // Retrieve payment
      cy.retrievePaymentCallTest({ globalState, data: confirmData });

      // Capture payment
      const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
      cy.captureCallTest(fixtures.captureBody, captureData, globalState);

      // Retrieve payment after capture
      cy.retrievePaymentCallTest({ globalState, data: captureData });
    });
  });

  context("Card - NoThreeDS Manual Partial Capture payment flow test", () => {
    it("should complete partial capture with separate create and confirm", () => {
      // Create payment intent
      const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
      cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "manual", globalState);

      // List payment methods
      cy.paymentMethodsCallTest(globalState);

      // Confirm payment
      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSManualCapture"];
      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

      if(!utils.should_continue_further(confirmData)) return;

      // Retrieve payment
      cy.retrievePaymentCallTest({ globalState, data: confirmData });

      // Partial capture
      const partialCaptureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];
      cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState);

      // Retrieve payment after partial capture
      cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
    });

    it("should complete partial capture with create+confirm", () => {
      // Create and confirm payment
      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSManualCapture"];
      cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "no_three_ds", "manual", globalState);

      if(!utils.should_continue_further(confirmData)) return;

      // Retrieve payment
      cy.retrievePaymentCallTest({ globalState, data: confirmData });

      // Partial capture
      const partialCaptureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];
      cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState);

      // Retrieve payment after partial capture
      cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
    });
  });
});