import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Payment Methods Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should create payment method for customer", () => {
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
    cy.createPaymentMethodTest(globalState, data);

    cy.listCustomerPMCallTest(globalState);
  });

  it("should set default payment method", () => {
    cy.listCustomerPMCallTest(globalState);

    const paymentMethodData =
      getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
    cy.createPaymentMethodTest(globalState, paymentMethodData);

    const createPaymentData = getConnectorDetails(
      globalState.get("connectorId")
    )["card_pm"]["PaymentIntentOffSession"];
    cy.createPaymentIntentTest(
      fixtures.createPaymentBody,
      createPaymentData,
      "no_three_ds",
      "automatic",
      globalState
    );

    const confirmData = getConnectorDetails(globalState.get("connectorId"))[
      "card_pm"
    ]["SaveCardUseNo3DSAutoCaptureOffSession"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    cy.listCustomerPMCallTest(globalState);

    cy.setDefaultPaymentMethodTest(globalState);
  });

  it("should delete payment method for customer", () => {
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
    cy.createPaymentMethodTest(globalState, data);

    cy.listCustomerPMCallTest(globalState);

    cy.deletePaymentMethodTest(globalState);
  });

  context("'Last Used' off-session token payments", () => {
    it("should create No 3DS off session save card payment", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SaveCardUseNo3DSAutoCaptureOffSession"];
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (!utils.should_continue_further(data)) return;

      cy.listCustomerPMCallTest(globalState);
    });

    it("should create 3DS off session save card payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SaveCardUse3DSAutoCaptureOffSession"];
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (!utils.should_continue_further(data)) return;

      const expectedRedirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expectedRedirection);

      cy.listCustomerPMCallTest(globalState);
    });

    it("should create 3DS off session save card payment with token", () => {
      const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

      const createData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        createData,
        "three_ds",
        "automatic",
        globalState
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SaveCardUseNo3DSAutoCapture"];
      const newData = {
        ...confirmData,
        Response: {
          ...confirmData.Response,
          body: {
            ...confirmData.Response.body,
            status: "requires_customer_action",
          },
        },
      };
      cy.saveCardConfirmCallTest(saveCardBody, newData, globalState);

      if (!utils.should_continue_further(newData)) return;

      const expectedRedirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expectedRedirection);

      cy.listCustomerPMCallTest(globalState, 1 /* order */);
    });

    it("should create No 3DS off session save card payment with token", () => {
      const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

      const createData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["PaymentIntent"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        createData,
        "no_three_ds",
        "automatic",
        globalState
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SaveCardUseNo3DSAutoCapture"];
      cy.saveCardConfirmCallTest(saveCardBody, confirmData, globalState);

      if (!utils.should_continue_further(confirmData)) return;

      cy.listCustomerPMCallTest(globalState);
    });
  });
});
