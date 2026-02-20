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

  it("Card - NoThreeDS Create and Confirm Automatic CIT and MIT payment flow test", () => {
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntentOffSession"];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      paymentIntentData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const citData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["PaymentMethodIdMandateNo3DSAutoCapture"];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      citData,
      6000,
      true,
      "automatic",
      "new_mandate",
      globalState
    );

    if (!utils.should_continue_further(citData)) return;

    cy.retrievePaymentCallTest({ globalState, data: citData });

    const mitData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MITAutoCapture"];

    cy.mitUsingPMId(
      fixtures.pmIdConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );

    cy.retrievePaymentCallTest({ globalState, data: mitData });
  });

  it("Card - NoThreeDS Create and Confirm Manual CIT and MIT payment flow test", () => {
    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntentOffSession"];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      paymentIntentData,
      "no_three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const citData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["PaymentMethodIdMandateNo3DSManualCapture"];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      citData,
      6000,
      true,
      "manual",
      "new_mandate",
      globalState
    );

    if (!utils.should_continue_further(citData)) return;

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: captureData });

    const mitData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MITAutoCapture"];

    cy.mitUsingPMId(
      fixtures.pmIdConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );

    cy.retrievePaymentCallTest({ globalState, data: mitData });
  });

  it("Card - NoThreeDS Create + Confirm Automatic CIT and MIT payment flow test", () => {
    const citData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["PaymentMethodIdMandateNo3DSAutoCapture"];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      citData,
      6000,
      true,
      "automatic",
      "new_mandate",
      globalState
    );

    if (!utils.should_continue_further(citData)) return;

    cy.retrievePaymentCallTest({ globalState, data: citData });

    const mitData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MITAutoCapture"];

    cy.mitUsingPMId(
      fixtures.pmIdConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );

    cy.retrievePaymentCallTest({ globalState, data: mitData });

    cy.mitUsingPMId(
      fixtures.pmIdConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );

    cy.retrievePaymentCallTest({ globalState, data: mitData });
  });

  it("Card - NoThreeDS Create + Confirm Manual CIT and MIT payment flow test", () => {
    const citData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["PaymentMethodIdMandateNo3DSManualCapture"];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      citData,
      6000,
      true,
      "manual",
      "new_mandate",
      globalState
    );

    if (!utils.should_continue_further(citData)) return;

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: captureData });

    const mitManualData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MITManualCapture"];

    cy.mitUsingPMId(
      fixtures.pmIdConfirmBody,
      mitManualData,
      6000,
      true,
      "manual",
      globalState
    );

    if (!utils.should_continue_further(mitManualData)) return;

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: captureData });

    cy.mitUsingPMId(
      fixtures.pmIdConfirmBody,
      mitManualData,
      6000,
      true,
      "manual",
      globalState
    );

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: captureData });
  });

  it("Card - MIT without billing address", () => {
    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntentOffSession"];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      paymentIntentData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const citData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["PaymentMethodIdMandateNo3DSAutoCapture"];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      citData,
      6000,
      true,
      "automatic",
      "new_mandate",
      globalState
    );

    if (!utils.should_continue_further(citData)) return;

    const mitData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MITWithoutBillingAddress"];

    cy.mitUsingPMId(
      fixtures.pmIdConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );
  });

  it("Card - ThreeDS Create + Confirm Automatic CIT and MIT payment flow test", () => {
    const citData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["PaymentMethodIdMandate3DSAutoCapture"];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      citData,
      6000,
      true,
      "automatic",
      "new_mandate",
      globalState
    );

    if (!utils.should_continue_further(citData)) return;

    const expected_redirection = fixtures.citConfirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    cy.retrievePaymentCallTest({ globalState, data: citData });

    const mitData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MITAutoCapture"];

    cy.mitUsingPMId(
      fixtures.pmIdConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );

    cy.mitUsingPMId(
      fixtures.pmIdConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );
  });

  it("Card - ThreeDS Create + Confirm Manual CIT and MIT payment flow", () => {
    const citData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["PaymentMethodIdMandate3DSManualCapture"];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      citData,
      6000,
      true,
      "manual",
      "new_mandate",
      globalState
    );

    if (!utils.should_continue_further(citData)) return;

    const expected_redirection = fixtures.citConfirmBody["return_url"];
    cy.handleRedirection(globalState, expected_redirection);

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: captureData });

    const mitData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MITAutoCapture"];

    cy.mitUsingPMId(
      fixtures.pmIdConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );
  });
});
