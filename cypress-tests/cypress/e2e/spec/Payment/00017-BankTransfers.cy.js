import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import { payment_methods_enabled } from "../../configs/Payment/Commons";

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

  // Dedicated setup context that runs before payment tests
  // Always creates fresh resources to ensure clean test state
  context("Setup", () => {
    it("create merchant", function () {
      // Skip if we already have a merchant connector set up
      if (globalState.get("merchantConnectorId")) {
        this.skip();
        return;
      }

      cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
    });

    it("create API key", function () {
      if (globalState.get("merchantConnectorId")) {
        this.skip();
        return;
      }

      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
    });

    it("create customer", function () {
      if (globalState.get("merchantConnectorId")) {
        this.skip();
        return;
      }

      // Create customer so payments have proper customer_id and email is returned in response
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("create connector", function () {
      if (globalState.get("merchantConnectorId")) {
        this.skip();
        return;
      }

      cy.createConnectorCallTest(
        "payment_processor",
        fixtures.createConnectorBody,
        payment_methods_enabled,
        globalState
      );
    });
  });

  context("Bank transfer - Pix forward flow", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("Pix");

      // Skip if this connector doesn't support this payment method
      if (!data) {
        shouldContinue = false;
        this.skip();
        return;
      }

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank transfer", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["Pix"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle bank transfer redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");

      cy.handleBankTransferRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });

  context("Bank transfer - Instant Bank Transfer Finland forward flow", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("InstantBankTransferFinland");

      // Skip if this connector doesn't support this payment method
      if (!data) {
        shouldContinue = false;
        this.skip();
        return;
      }

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank transfer", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["InstantBankTransferFinland"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle bank transfer redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");

      cy.handleBankTransferRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });

  context("Bank transfer - Instant Bank Transfer Poland forward flow", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("InstantBankTransferPoland");

      // Skip if this connector doesn't support this payment method
      if (!data) {
        shouldContinue = false;
        this.skip();
        return;
      }

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank transfer", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["InstantBankTransferPoland"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle bank transfer redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");

      cy.handleBankTransferRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });

  context("Bank transfer - Ach flow", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("Ach");

      // Skip if this connector doesn't support this payment method
      if (!data) {
        shouldContinue = false;
        this.skip();
        return;
      }

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank transfer", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["Ach"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle bank transfer redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");

      if (globalState.get("connectorId") != "checkbook") {
        cy.handleBankTransferRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      }
    });
  });

  context("Bank transfer - LocalBankTransfer flow (PeachPayments APM)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", function () {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["PaymentIntent"]("LocalBankTransfer");

      // Skip if this connector doesn't support this payment method
      if (!data) {
        shouldContinue = false;
        this.skip();
        return;
      }

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank transfer", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["LocalBankTransfer"];

      cy.confirmBankTransferCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Handle bank transfer redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");

      // Skip redirect handling for peachpaymentsapm - redirects to external PeachPayments page
      // which cannot be automated. The Confirm test already validates the redirect URL is returned.
      if (globalState.get("connectorId") === "peachpaymentsapm") {
        cy.log("Skipping redirect handling - external PeachPayments page cannot be automated");
        return;
      }

      cy.handleBankTransferRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });
});
