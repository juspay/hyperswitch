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

  it("Card - NoThreeDS Create + Confirm Automatic CIT and Single use MIT payment flow test", () => {
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const citData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["ZeroAuthMandate"];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      citData,
      0,
      true,
      "automatic",
      "setup_mandate",
      globalState
    );

    if (!utils.should_continue_further(citData)) return;

    cy.retrievePaymentCallTest({ globalState, data: citData });

    const mitData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MITAutoCapture"];

    cy.mitForMandatesCallTest(
      fixtures.mitConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );

    cy.retrievePaymentCallTest({ globalState, data: mitData });
  });

  it("Card - NoThreeDS Create + Confirm Automatic CIT and Multi use MIT payment flow test", () => {
    const citData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["ZeroAuthMandate"];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      citData,
      0,
      true,
      "automatic",
      "setup_mandate",
      globalState
    );

    if (!utils.should_continue_further(citData)) return;

    cy.retrievePaymentCallTest({ globalState, data: citData });

    const mitData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["MITAutoCapture"];

    cy.mitForMandatesCallTest(
      fixtures.mitConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );

    cy.retrievePaymentCallTest({ globalState, data: mitData });

    cy.mitForMandatesCallTest(
      fixtures.mitConfirmBody,
      mitData,
      6000,
      true,
      "automatic",
      globalState
    );

    cy.retrievePaymentCallTest({ globalState, data: mitData });
  });

  it("Card - Zero Auth Payment", () => {
    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["ZeroAuthPaymentIntent"];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      paymentIntentData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["ZeroAuthConfirmPayment"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    cy.listCustomerPMCallTest(globalState);

    const recurringIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntentOffSession"];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      recurringIntentData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(recurringIntentData)) return;

    const saveCardData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SaveCardConfirmAutoCaptureOffSession"];

    cy.saveCardConfirmCallTest(
      fixtures.saveCardConfirmBody,
      saveCardData,
      globalState
    );

    if (!utils.should_continue_further(saveCardData)) return;

    cy.retrievePaymentCallTest({ globalState, data: saveCardData });
  });

  it("Card - Zero auth Mandate flow Using PMID (create and confirm)", () => {
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["ZeroAuthPaymentIntent"];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      paymentIntentData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["ZeroAuthConfirmPayment"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    const retrieveData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["ZeroAuthMandate"];

    cy.retrievePaymentCallTest({ globalState, data: retrieveData });

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

  it("Card - Zero auth Mandate flow Using PMID (create + confirm)", () => {
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const citData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["ZeroAuthConfirmPayment"];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      citData,
      0,
      true,
      "automatic",
      "setup_mandate",
      globalState
    );

    if (!utils.should_continue_further(citData)) return;

    const retrieveData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["ZeroAuthMandate"];

    cy.retrievePaymentCallTest({ globalState, data: retrieveData });

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
});
