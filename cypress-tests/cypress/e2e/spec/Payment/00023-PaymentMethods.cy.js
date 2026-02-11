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
    // Create customer
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    // Create Payment Method
    const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
    cy.createPaymentMethodTest(globalState, data);

    // List PM for customer
    cy.listCustomerPMCallTest(globalState);
  });

  it("should set default payment method", () => {
    // List PM for customer
    cy.listCustomerPMCallTest(globalState);

    // Create Payment Method
    const paymentMethodData = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
    cy.createPaymentMethodTest(globalState, paymentMethodData);

    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntentOffSession"];
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "no_three_ds", "automatic", globalState);

    // Confirm payment
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCaptureOffSession"];
    cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);

    // List PM for customer
    cy.listCustomerPMCallTest(globalState);

    // Set default payment method
    cy.setDefaultPaymentMethodTest(globalState);
  });

  it("should delete payment method for customer", () => {
    // Create customer
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

    // Create Payment Method
    const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
    cy.createPaymentMethodTest(globalState, data);

    // List PM for customer
    cy.listCustomerPMCallTest(globalState);

    // Delete Payment Method for a customer
    cy.deletePaymentMethodTest(globalState);
  });

  context("'Last Used' off-session token payments", () => {
    it("should create No 3DS off session save card payment", () => {
      // Create customer
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

      // Create and confirm payment
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCaptureOffSession"];
      cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, data, "no_three_ds", "automatic", globalState);

      // List PM for customer
      cy.listCustomerPMCallTest(globalState);
    });

    it("should create 3DS off session save card payment", () => {
      // Create and confirm payment
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUse3DSAutoCaptureOffSession"];
      cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, data, "three_ds", "automatic", globalState);

      // Handle redirection
      const expectedRedirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expectedRedirection);

      // List PM for customer
      cy.listCustomerPMCallTest(globalState);
    });

    it("should create 3DS off session save card payment with token", () => {
      const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

      // Create payment intent
      const createData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
      cy.createPaymentIntentTest(fixtures.createPaymentBody, createData, "three_ds", "automatic", globalState);

      // Confirm save card payment
      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCapture"];
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

      // Handle redirection
      const expectedRedirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expectedRedirection);

      // List PM for customer
      cy.listCustomerPMCallTest(globalState, 1 /* order */);
    });

    it("should create No 3DS off session save card payment with token", () => {
      const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);

      // Create payment intent
      const createData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
      cy.createPaymentIntentTest(fixtures.createPaymentBody, createData, "no_three_ds", "automatic", globalState);

      // Confirm save card payment
      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SaveCardUseNo3DSAutoCapture"];
      cy.saveCardConfirmCallTest(saveCardBody, confirmData, globalState);

      // List PM for customer
      cy.listCustomerPMCallTest(globalState);
    });
  });
});