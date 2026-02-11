import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Refund flow - No 3DS", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should fully refund No-3DS payment", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Full refund
    const refundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
    cy.refundCallTest(fixtures.refundBody, refundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("should partially refund No-3DS payment", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Partial refunds
    const partialRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("should fully refund No-3DS payment with create+confirm", () => {
    // Create and confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
    cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "no_three_ds", "automatic", globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Full refund
    const refundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
    cy.refundCallTest(fixtures.refundBody, refundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("should partially refund No-3DS payment with create+confirm", () => {
    // Create and confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
    cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "no_three_ds", "automatic", globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Partial refunds
    const partialRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    const newData = { ...syncRefundData, Response: syncRefundData.ResponseCustom || syncRefundData.Response };
    cy.refundCallTest(fixtures.refundBody, newData, globalState);
  });

  it("should fully refund fully captured No-3DS payment", () => {
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

    // Capture payment
    const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Retrieve payment after capture
    cy.retrievePaymentCallTest({ globalState, data: captureData });

    // Full refund
    const refundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["manualPaymentRefund"];
    const newRefundData = { ...refundData, Response: refundData.ResponseCustom || refundData.Response };
    cy.refundCallTest(fixtures.refundBody, newRefundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("should partially refund fully captured No-3DS payment", () => {
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

    // Capture payment
    const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Retrieve payment after capture
    cy.retrievePaymentCallTest({ globalState, data: captureData });

    // Partial refunds
    const partialRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["manualPaymentPartialRefund"];
    const newPartialRefundData = { ...partialRefundData, Response: partialRefundData.ResponseCustom || partialRefundData.Response };
    cy.refundCallTest(fixtures.refundBody, newPartialRefundData, globalState);
    cy.refundCallTest(fixtures.refundBody, newPartialRefundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);

    // List refunds
    cy.listRefundCallTest(fixtures.listRefundCall, globalState);
  });

  it("should fully refund partially captured No-3DS payment", () => {
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

    // Partial capture
    const partialCaptureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];
    cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState);

    // Retrieve payment after capture
    cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });

    // Full refund
    const refundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["manualPaymentPartialRefund"];
    const newRefundData = { ...refundData, Response: refundData.ResponseCustom || refundData.Response };
    cy.refundCallTest(fixtures.refundBody, newRefundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("should partially refund partially captured No-3DS payment", () => {
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

    // Partial capture
    const partialCaptureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];
    cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState);

    // Retrieve payment after capture
    cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });

    // Partial refund
    const partialRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["manualPaymentPartialRefund"];
    const newPartialRefundData = { ...partialRefundData, Response: partialRefundData.ResponseCustom || partialRefundData.Response };
    cy.refundCallTest(fixtures.refundBody, newPartialRefundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("should fully refund CIT and MIT payment flow", () => {
    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateMultiUseNo3DSAutoCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "automatic", "new_mandate", globalState);

    // Confirm No 3DS MIT (first)
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "automatic", globalState);

    // Confirm No 3DS MIT (second)
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "automatic", globalState);

    // Full refund
    const refundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
    cy.refundCallTest(fixtures.refundBody, refundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });
});

describe("Card - Refund flow - 3DS", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  it("should fully refund 3DS payment", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm 3DS
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSAutoCapture"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return; 

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Full refund
    const refundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
    cy.refundCallTest(fixtures.refundBody, refundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("should partially refund 3DS payment", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm 3DS
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSAutoCapture"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return; 

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Partial refunds
    const partialRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("should fully refund 3DS payment with create+confirm", () => {
    // Create and confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSAutoCapture"];
    cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "three_ds", "automatic", globalState);

    if (!utils.should_continue_further(confirmData)) return; 

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Full refund
    const refundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Refund"];
    cy.refundCallTest(fixtures.refundBody, refundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("should partially refund 3DS payment with create+confirm", () => {
    // Create and confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSAutoCapture"];
    cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "three_ds", "automatic", globalState);

    if (!utils.should_continue_further(confirmData)) return;

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // Partial refunds
    const partialRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("should fully refund fully captured 3DS payment", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "manual", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm 3DS
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

    // Full refund
    const refundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["manualPaymentRefund"];
    cy.refundCallTest(fixtures.refundBody, refundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("should partially refund fully captured 3DS payment", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "manual", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm 3DS
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

    // Partial refunds
    const partialRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["manualPaymentPartialRefund"];
    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);
    cy.refundCallTest(fixtures.refundBody, partialRefundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("should fully refund partially captured 3DS payment", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "manual", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm 3DS
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

    // Full refund
    const refundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["manualPaymentPartialRefund"];
    cy.refundCallTest(fixtures.refundBody, refundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });

  it("should partially refund partially captured 3DS payment", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "manual", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm 3DS
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

    // Partial refund
    const partialRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["manualPaymentPartialRefund"];
    const newPartialRefundData = { ...partialRefundData, Request: { amount: partialRefundData.Request.amount / 2 } };
    cy.refundCallTest(fixtures.refundBody, newPartialRefundData, globalState);

    // Sync refund
    const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
    cy.syncRefundCallTest(syncRefundData, globalState);
  });
});