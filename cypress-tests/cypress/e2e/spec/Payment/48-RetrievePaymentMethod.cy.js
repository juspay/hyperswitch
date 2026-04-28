import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Retrieve Payment Method Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Retrieve single payment method", () => {
    it("Create customer -> Create Payment Method -> Retrieve Payment Method", () => {
      cy.step("Create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      cy.step("Create Payment Method", () => {
        const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
        cy.createPaymentMethodTest(globalState, data);
      });

      cy.step("Retrieve Payment Method", () => {
        cy.retrievePaymentMethodTest(globalState);
      });
    });
  });

  context("Retrieve payment method with full card details", () => {
    it("Create customer -> Create Payment Method -> Retrieve and verify card details", () => {
      let shouldContinue = true;

      cy.step("Create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      cy.step("Create Payment Method", () => {
        const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
        cy.createPaymentMethodTest(globalState, data);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve and verify payment method details", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve and verify payment method details");
          return;
        }
        cy.retrievePaymentMethodTest(globalState);
      });

      cy.step("List to confirm retrieval consistency", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List to confirm retrieval consistency");
          return;
        }
        cy.listCustomerPMCallTest(globalState);
      });
    });
  });

  context("Retrieve payment method after payment flow", () => {
    it("Create payment with saved card -> Retrieve the saved payment method", () => {
      let shouldContinue = true;

      cy.step("Create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      cy.step("Create+Confirm payment with saved card", () => {
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

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List PM for customer", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List PM for customer");
          return;
        }
        cy.listCustomerPMCallTest(globalState);
      });

      cy.step("Retrieve specific payment method", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve specific payment method");
          return;
        }
        cy.retrievePaymentMethodTest(globalState);
      });
    });
  });

  context("Retrieve non-existent payment method", () => {
    it("Attempt to retrieve deleted payment method should return 404", () => {
      let shouldContinue = true;

      cy.step("Create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      cy.step("Create Payment Method", () => {
        const data = getConnectorDetails("commons")["card_pm"]["PaymentMethod"];
        cy.createPaymentMethodTest(globalState, data);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Store payment method ID and delete", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Store payment method ID and delete");
          return;
        }
        // Payment method ID is already in globalState from createPaymentMethodTest
        cy.deletePaymentMethodTest(globalState);
      });

      cy.step("Attempt to retrieve deleted payment method", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Attempt to retrieve deleted payment method");
          return;
        }
        // This should return 404 since the payment method was deleted
        cy.retrievePaymentMethodTest(globalState);
      });
    });
  });
});
