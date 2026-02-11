import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Bank Redirect tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should complete Blik bank redirect flow", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["PaymentIntent"]("Blik");
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm bank redirect
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["Blik"];
    cy.confirmBankRedirectCallTest(fixtures.confirmBody, confirmData, true, globalState);
  });

  it("should complete EPS bank redirect flow", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["PaymentIntent"]("Eps");
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm bank redirect
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["Eps"];
    cy.confirmBankRedirectCallTest(fixtures.confirmBody, confirmData, true, globalState);

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");
    cy.handleBankRedirectRedirection(globalState, payment_method_type, expected_redirection);
  });

  it("should complete iDEAL bank redirect flow", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["PaymentIntent"]("Ideal");
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm bank redirect
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["Ideal"];
    cy.confirmBankRedirectCallTest(fixtures.confirmBody, confirmData, true, globalState);

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");
    cy.handleBankRedirectRedirection(globalState, payment_method_type, expected_redirection);
  });

  it("should complete Sofort bank redirect flow", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["PaymentIntent"]("Sofort");
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm bank redirect
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["Sofort"];
    cy.confirmBankRedirectCallTest(fixtures.confirmBody, confirmData, true, globalState);

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");
    cy.handleBankRedirectRedirection(globalState, payment_method_type, expected_redirection);
  });

  it("should complete Przelewy24 bank redirect flow", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["PaymentIntent"]("Przelewy24");
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm bank redirect
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["Przelewy24"];
    cy.confirmBankRedirectCallTest(fixtures.confirmBody, confirmData, true, globalState);

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");
    cy.handleBankRedirectRedirection(globalState, payment_method_type, expected_redirection);
  });

  it("should complete OpenBankingUk bank redirect flow", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["PaymentIntent"]("OpenBankingUk");
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm bank redirect
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["OpenBankingUk"];
    cy.confirmBankRedirectCallTest(fixtures.confirmBody, confirmData, true, globalState);

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");
    cy.handleBankRedirectRedirection(globalState, payment_method_type, expected_redirection);

    // Sync payment status
    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });

  it("should complete OnlineBankingFpx bank redirect flow", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["PaymentIntent"]("OnlineBankingFpx");
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm bank redirect
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["OnlineBankingFpx"];
    cy.confirmBankRedirectCallTest(fixtures.confirmBody, confirmData, true, globalState);

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");
    cy.handleBankRedirectRedirection(globalState, payment_method_type, expected_redirection);

    // Sync payment status
    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });

  it("should complete Interac bank redirect flow", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["PaymentIntent"]("Interac");
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm bank redirect
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["bank_redirect_pm"]["Interac"];
    cy.confirmBankRedirectCallTest(fixtures.confirmBody, confirmData, true, globalState);

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");
    cy.handleBankRedirectRedirection(globalState, payment_method_type, expected_redirection);

    // Sync payment status
    cy.retrievePaymentCallTest({ globalState, data: confirmData });
  });
});