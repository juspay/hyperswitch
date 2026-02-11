import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Bank Transfers", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should complete Pix bank transfer flow", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["bank_transfer_pm"]["PaymentIntent"]("Pix");
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm bank transfer
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["bank_transfer_pm"]["Pix"];
    cy.confirmBankTransferCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");
    cy.handleBankTransferRedirection(globalState, payment_method_type, expected_redirection);
  });

  it("should complete Instant Bank Transfer Finland flow", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["bank_transfer_pm"]["PaymentIntent"]("InstantBankTransferFinland");
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm bank transfer
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["bank_transfer_pm"]["InstantBankTransferFinland"];
    cy.confirmBankTransferCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");
    cy.handleBankTransferRedirection(globalState, payment_method_type, expected_redirection);
  });

  it("should complete Instant Bank Transfer Poland flow", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["bank_transfer_pm"]["PaymentIntent"]("InstantBankTransferPoland");
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm bank transfer
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["bank_transfer_pm"]["InstantBankTransferPoland"];
    cy.confirmBankTransferCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    // Handle redirection
    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");
    cy.handleBankTransferRedirection(globalState, payment_method_type, expected_redirection);
  });

  it("should complete Ach bank transfer flow", () => {
    // Create payment intent
    const createPaymentData = getConnectorDetails(globalState.get("connectorId"))["bank_transfer_pm"]["PaymentIntent"]("Ach");
    cy.createPaymentIntentTest(fixtures.createPaymentBody, createPaymentData, "three_ds", "automatic", globalState);

    // List payment methods
    cy.paymentMethodsCallTest(globalState);

    // Confirm bank transfer
    const confirmData = getConnectorDetails(globalState.get("connectorId"))["bank_transfer_pm"]["Ach"];
    cy.confirmBankTransferCallTest(fixtures.confirmBody, confirmData, true, globalState);

    if (!utils.should_continue_further(confirmData)) return;

    // Handle redirection (skip for checkbook connector)
    const expected_redirection = fixtures.confirmBody["return_url"];
    const payment_method_type = globalState.get("paymentMethodType");
    if (globalState.get("connectorId") !== "checkbook") {
      cy.handleBankTransferRedirection(globalState, payment_method_type, expected_redirection);
    }
  });
});