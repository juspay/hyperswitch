import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Mandates using Payment Method Id flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should complete NoThreeDS create and confirm automatic CIT and MIT payment flow", () => {
    // Create customer
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    // Create No 3DS Payment Intent
    const paymentIntentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntentOffSession"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, paymentIntentData, "no_three_ds", "automatic", globalState);

    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentMethodIdMandateNo3DSAutoCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "automatic", "new_mandate", globalState);

    if(!utils.should_continue_further(citData)) return;

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: citData });

    // Confirm No 3DS MIT
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitUsingPMId(fixtures.pmIdConfirmBody, mitData, 6000, true, "automatic", globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: mitData });
  });

  it("should complete NoThreeDS create and confirm manual CIT and MIT payment flow", () => {
    // Create No 3DS Payment Intent
    const paymentIntentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntentOffSession"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, paymentIntentData, "no_three_ds", "manual", globalState);

    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentMethodIdMandateNo3DSManualCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "manual", "new_mandate", globalState);

    if(!utils.should_continue_further(citData)) return;

    // Capture CIT
    const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: captureData });

    // Confirm No 3DS MIT
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitUsingPMId(fixtures.pmIdConfirmBody, mitData, 6000, true, "automatic", globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: mitData });
  });

  it("should complete NoThreeDS create+confirm automatic CIT and MIT payment flow", () => {
    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentMethodIdMandateNo3DSAutoCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "automatic", "new_mandate", globalState);

    if(!utils.should_continue_further(citData)) return;

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: citData });

    // Confirm No 3DS MIT (first)
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitUsingPMId(fixtures.pmIdConfirmBody, mitData, 6000, true, "automatic", globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: mitData });

    // Confirm No 3DS MIT (second)
    cy.mitUsingPMId(fixtures.pmIdConfirmBody, mitData, 6000, true, "automatic", globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: mitData });
  });

  it("should complete NoThreeDS create+confirm manual CIT and MIT payment flow", () => {
    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentMethodIdMandateNo3DSManualCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "manual", "new_mandate", globalState);

    if(!utils.should_continue_further(citData)) return;

    // Capture CIT
    const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: captureData });

    // Confirm No 3DS MIT 1
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITManualCapture"];
    cy.mitUsingPMId(fixtures.pmIdConfirmBody, mitData, 6000, true, "manual", globalState);

    // Capture MIT 1
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: captureData });

    // Confirm No 3DS MIT 2
    cy.mitUsingPMId(fixtures.pmIdConfirmBody, mitData, 6000, true, "manual", globalState);

    // Capture MIT 2
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: captureData });
  });

  it("should complete MIT without billing address", () => {
    // Create No 3DS Payment Intent
    const paymentIntentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntentOffSession"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, paymentIntentData, "no_three_ds", "automatic", globalState);

    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentMethodIdMandateNo3DSAutoCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "automatic", "new_mandate", globalState);

    if(!utils.should_continue_further(citData)) return;

    // Confirm No 3DS MIT without billing address
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITWithoutBillingAddress"];
    cy.mitUsingPMId(fixtures.pmIdConfirmBody, mitData, 6000, true, "automatic", globalState);
  });

  it("should complete ThreeDS create+confirm automatic CIT and MIT payment flow", () => {
    // Confirm 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentMethodIdMandate3DSAutoCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "automatic", "new_mandate", globalState);

    if (!utils.should_continue_further(citData)) return;

    // Handle redirection
    const expected_redirection = fixtures.citConfirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: citData });

    // Confirm No 3DS MIT (first)
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitUsingPMId(fixtures.pmIdConfirmBody, mitData, 6000, true, "automatic", globalState);

    // Confirm No 3DS MIT (second)
    cy.mitUsingPMId(fixtures.pmIdConfirmBody, mitData, 6000, true, "automatic", globalState);
  });

  it("should complete ThreeDS create+confirm manual CIT and MIT payment flow", () => {
    // Confirm 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentMethodIdMandate3DSManualCapture"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "manual", "new_mandate", globalState);

    if (!utils.should_continue_further(citData)) return;

    // Handle redirection
    const expected_redirection = fixtures.citConfirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    // Capture CIT
    const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: captureData });

    // Confirm No 3DS MIT
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitUsingPMId(fixtures.pmIdConfirmBody, mitData, 6000, true, "automatic", globalState);
  });
});