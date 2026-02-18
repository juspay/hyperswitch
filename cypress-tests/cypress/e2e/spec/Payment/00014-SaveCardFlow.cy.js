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

  it("Save card for NoThreeDS automatic capture payment- Create+Confirm [on_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const createConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSAutoCapture"];

    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      createConfirmData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(createConfirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: createConfirmData });

    cy.listCustomerPMCallTest(globalState);

    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntent"];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      paymentIntentData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const saveCardConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSAutoCapture"];

    cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState);
  });

  it("Save card for NoThreeDS manual full capture payment- Create+Confirm [on_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const createConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSAutoCapture"];

    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      createConfirmData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(createConfirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: createConfirmData });

    cy.listCustomerPMCallTest(globalState);

    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntent"];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      paymentIntentData,
      "no_three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const saveCardConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSManualCapture"];

    cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState);

    if (!utils.should_continue_further(saveCardConfirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: saveCardConfirmData });

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: captureData });
  });

  it("Save card for NoThreeDS manual partial capture payment- Create + Confirm [on_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const createConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSAutoCapture"];

    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      createConfirmData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(createConfirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: createConfirmData });

    cy.listCustomerPMCallTest(globalState);

    const paymentIntentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntent"];

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      paymentIntentData,
      "no_three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const saveCardConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSManualCapture"];

    cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState);

    if (!utils.should_continue_further(saveCardConfirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: saveCardConfirmData });

    const partialCaptureData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PartialCapture"];

    cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState);

    if (!utils.should_continue_further(partialCaptureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
  });

  it("Save card for NoThreeDS automatic capture payment [off_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const createConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSAutoCaptureOffSession"];

    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      createConfirmData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(createConfirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: createConfirmData });

    cy.listCustomerPMCallTest(globalState);

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

    const saveCardConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardConfirmAutoCaptureOffSession"];

    cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState);
  });

  it("Save card for NoThreeDS manual capture payment- Create+Confirm [off_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const createConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSManualCaptureOffSession"];

    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      createConfirmData,
      "no_three_ds",
      "manual",
      globalState
    );

    if (!utils.should_continue_further(createConfirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: createConfirmData });

    const captureData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["Capture"];

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: captureData });

    cy.listCustomerPMCallTest(globalState);

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

    const saveCardConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardConfirmManualCaptureOffSession"];

    cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState);

    if (!utils.should_continue_further(saveCardConfirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: saveCardConfirmData });

    cy.captureCallTest(fixtures.captureBody, captureData, globalState);

    if (!utils.should_continue_further(captureData)) return;

    cy.retrievePaymentCallTest({ globalState, data: captureData });
  });

  it("Save card for NoThreeDS automatic capture payment - create and confirm [off_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

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

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SaveCardUseNo3DSAutoCaptureOffSession"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.retrievePaymentCallTest({ globalState, data: confirmData });

    cy.listCustomerPMCallTest(globalState);

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      paymentIntentData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const saveCardConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardConfirmAutoCaptureOffSession"];

    cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState);
  });

  it("Use billing address from payment method during subsequent payment [off_session]", () => {
    const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

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

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SaveCardUseNo3DSAutoCaptureOffSession"];

    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.listCustomerPMCallTest(globalState);

    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      paymentIntentData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(paymentIntentData)) return;

    const saveCardConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardConfirmAutoCaptureOffSessionWithoutBilling"];

    cy.saveCardConfirmCallTest(saveCardBody, saveCardConfirmData, globalState);
  });

  it("Check if card fields are populated when saving card again after a metadata update", () => {
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const createConfirmData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["SaveCardUseNo3DSAutoCapture"];

    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      createConfirmData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(createConfirmData)) return;

    cy.listCustomerPMCallTest(globalState);

    const card_holder_name = generateRandomName();
    const newData = {
      ...createConfirmData,
      Request: {
        ...createConfirmData.Request,
        payment_method_data: {
          card: {
            ...createConfirmData.Request.payment_method_data.card,
            card_holder_name: card_holder_name,
          },
        },
      },
    };

    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      newData,
      "no_three_ds",
      "automatic",
      globalState
    );

    if (!utils.should_continue_further(createConfirmData)) return;

    cy.listCustomerPMCallTest(globalState);
  });
});
