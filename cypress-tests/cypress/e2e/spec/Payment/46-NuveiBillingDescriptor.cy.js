import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Nuvei - Billing Descriptor Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Billing Descriptor - Happy Path with Auto Capture", () => {
    it("Create Payment Intent -> Confirm Payment with billing_descriptor -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment with billing_descriptor", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with billing_descriptor");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["BillingDescriptorNo3DSAutoCapture"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["BillingDescriptorNo3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Billing Descriptor - Invalid Phone (Exceeds 13 chars)", () => {
    it("Create Payment Intent -> Confirm Payment with invalid billing_descriptor phone -> Payment fails with billing descriptor error", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment with invalid billing_descriptor phone", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with invalid billing_descriptor phone");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["BillingDescriptorInvalidPhone"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Validate payment failed with billing descriptor error", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Validate payment failed with billing descriptor error");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["BillingDescriptorInvalidPhone"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Billing Descriptor - Empty Descriptor", () => {
    it("Create Payment Intent -> Confirm Payment with empty billing_descriptor -> Payment succeeds with empty descriptor", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm Payment with empty billing_descriptor", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with empty billing_descriptor");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["BillingDescriptorEmptyDescriptor"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment with empty billing_descriptor", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment with empty billing_descriptor");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["BillingDescriptorEmptyDescriptor"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });
});
