import * as fixtures from "../../../fixtures/imports";
import { generateRandomName } from "../../../utils/RequestBodyUtils";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - SaveCard payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should save card for NoThreeDS automatic capture payment [on_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    // Create customer
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    // Create and confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCapture"];
    cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "no_three_ds", "automatic", globalState);

    if(!utils.should_continue_further(confirmData)) return;

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // List customer payment methods
    cy.listCustomerPMCallTest(globalState);

    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "automatic", globalState);

    // Confirm with saved card
    cy.saveCardConfirmCallTest(saveCardBody, confirmData, globalState);
  });

  it("should save card for NoThreeDS manual full capture payment [on_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    // Create customer
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    // Create and confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCapture"];
    cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "no_three_ds", "automatic", globalState);

    if(!utils.should_continue_further(confirmData)) return;

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // List customer payment methods
    cy.listCustomerPMCallTest(globalState);

    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "manual", globalState);

    // Confirm with saved card
    const saveCardManualData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSManualCapture"];
    cy.saveCardConfirmCallTest(saveCardBody, saveCardManualData, globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: saveCardManualData });

    // Capture payment
    const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Retrieve payment after capture
    cy.retrievePaymentCallTest({ globalState, data: captureData });
  });

  it("should save card for NoThreeDS manual partial capture payment [on_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    // Create customer
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    // Create and confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCapture"];
    cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "no_three_ds", "automatic", globalState);

    if(!utils.should_continue_further(confirmData)) return;

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // List customer payment methods
    cy.listCustomerPMCallTest(globalState);

    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "manual", globalState);

    // Confirm with saved card
    const saveCardManualData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSManualCapture"];
    cy.saveCardConfirmCallTest(saveCardBody, saveCardManualData, globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: saveCardManualData });

    // Partial capture
    const partialCaptureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];
    cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState);

    // Retrieve payment after capture
    cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
  });

  it("should save card for NoThreeDS automatic capture payment [off_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    // Create customer
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    // Create and confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCaptureOffSession"];
    cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "no_three_ds", "automatic", globalState);

    if(!utils.should_continue_further(confirmData)) return;

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // List customer payment methods
    cy.listCustomerPMCallTest(globalState);

    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntentOffSession"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "automatic", globalState);

    // Confirm with saved card
    const saveCardConfirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardConfirmAutoCaptureOffSession"];
    cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState);
  });

  it("should save card for NoThreeDS manual capture payment [off_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    // Create customer
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    // Create and confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSManualCaptureOffSession"];
    cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "no_three_ds", "manual", globalState);

    if(!utils.should_continue_further(confirmData)) return;

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Capture payment
    const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Retrieve payment after capture
    cy.retrievePaymentCallTest({ globalState, data: captureData });

    // List customer payment methods
    cy.listCustomerPMCallTest(globalState);

    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntentOffSession"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "manual", globalState);

    // Confirm with saved card
    const saveCardConfirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardConfirmManualCaptureOffSession"];
    cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: saveCardConfirmData });

    // Capture payment
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Retrieve payment after capture
    cy.retrievePaymentCallTest({ globalState, data: captureData });
  });

  it("should save card for NoThreeDS automatic capture payment with create and confirm [off_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    // Create customer
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntentOffSession"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "automatic", globalState);

    // Confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCaptureOffSession"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if(!utils.should_continue_further(confirmData)) return;

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // List customer payment methods
    cy.listCustomerPMCallTest(globalState);

    // Create payment intent
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "automatic", globalState);

    // Confirm with saved card
    const saveCardConfirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardConfirmAutoCaptureOffSession"];
    cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState);
  });

  it("should use billing address from payment method during subsequent payment [off_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    // Create customer
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntentOffSession"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "automatic", globalState);

    // Confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCaptureOffSession"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if(!utils.should_continue_further(confirmData)) return;

    // List customer payment methods
    cy.listCustomerPMCallTest(globalState);

    // Create payment intent
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "automatic", globalState);

    // Confirm with saved card without billing
    const saveCardConfirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardConfirmAutoCaptureOffSessionWithoutBilling"];
    cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState);
  });

  it("should populate card fields when saving card again after metadata update", () => {
    // Create customer
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    // Create and confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCapture"];
    cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "no_three_ds", "automatic", globalState);

    if(!utils.should_continue_further(confirmData)) return;

    // List customer payment methods
    cy.listCustomerPMCallTest(globalState);

    // Create and confirm payment with updated card holder name
    const card_holder_name = generateRandomName();
    const newData = {
      ...confirmData,
      Request: {
        ...confirmData.Request,
        payment_method_data: {
          card: {
            ...confirmData.Request.payment_method_data.card,
            card_holder_name: card_holder_name,
          },
        },
      },
    };
    cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, newData, "no_three_ds", "automatic", globalState);

    // List customer payment methods
    cy.listCustomerPMCallTest(globalState);
  });
});