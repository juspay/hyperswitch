import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - SingleUse Mandates flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should complete NoThreeDS automatic CIT and single use MIT payment flow", () => {
    // Create customer
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ZeroAuthMandate"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 0, true, "automatic", "setup_mandate", globalState);

    if(!utils.should_continue_further(citData)) return;

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: citData });

    // Confirm No 3DS MIT
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "automatic", globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: mitData });
  });

  it("should complete NoThreeDS automatic CIT and multi use MIT payment flow", () => {
    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ZeroAuthMandate"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 0, true, "automatic", "setup_mandate", globalState);

    if(!utils.should_continue_further(citData)) return;

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: citData });

    // Confirm No 3DS MIT (first)
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "automatic", globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: mitData });

    // Confirm No 3DS MIT (second)
    cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 6000, true, "automatic", globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: mitData });
  });

  it("should complete zero auth payment flow", () => {
    // Create No 3DS Payment Intent
    const paymentIntentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ZeroAuthPaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, paymentIntentData, "no_three_ds", "automatic", globalState);

    // Confirm No 3DS payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ZeroAuthConfirmPayment"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if(!utils.should_continue_further(confirmData)) return;

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    // List customer payment methods
    cy.listCustomerPMCallTest(globalState);

    // Create recurring payment intent
    const recurringPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntentOffSession"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, recurringPaymentData, "no_three_ds", "automatic", globalState);

    // Confirm recurring payment
    const saveCardData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardConfirmAutoCaptureOffSession"];
    cy.saveCardConfirmCallTest(fixtures.saveCardConfirmBody, saveCardData, globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: saveCardData });
  });

  it("should complete zero auth mandate flow using PMID with create and confirm", () => {
    // Create customer
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    // Create No 3DS Payment Intent
    const paymentIntentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ZeroAuthPaymentIntent"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, paymentIntentData, "no_three_ds", "automatic", globalState);

    // Confirm No 3DS payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ZeroAuthConfirmPayment"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if(!utils.should_continue_further(confirmData)) return;

    // Retrieve payment
    const zeroAuthData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ZeroAuthMandate"];
    cy.retrievePaymentCallTest({ globalState, data: zeroAuthData });

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

  it("should complete zero auth mandate flow using PMID with create+confirm", () => {
    // Create customer
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    // Confirm No 3DS CIT
    const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ZeroAuthConfirmPayment"];
    cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 0, true, "automatic", "setup_mandate", globalState);

    if(!utils.should_continue_further(citData)) return;

    // Retrieve payment
    const zeroAuthData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ZeroAuthMandate"];
    cy.retrievePaymentCallTest({ globalState, data: zeroAuthData });

    // Confirm No 3DS MIT
    const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
    cy.mitUsingPMId(fixtures.pmIdConfirmBody, mitData, 6000, true, "automatic", globalState);

    // Retrieve payment
    cy.retrievePaymentCallTest({ globalState, data: mitData });
  });
});