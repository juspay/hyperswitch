import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("UPI Payments - Hyperswitch", () => {
  context("[Payment] [UPI - UPI Collect] Create & Confirm + Refund", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create payment intent -> List Merchant payment methods -> Confirm payment -> Handle UPI Redirection -> Retrieve payment -> Refund payment", () => {
      let shouldContinue = true;

      cy.step("Create payment intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "upi_pm"
        ]["PaymentIntent"];
        // Use no_three_ds for Razorpay UPI as it doesn't support 3DS authentication
        const authType = globalState.get("connectorId") === "razorpay" ? "no_three_ds" : "three_ds";

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          authType,
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant payment methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant payment methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "upi_pm"
        ]["UpiCollect"];

        cy.confirmUpiCall(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle UPI Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle UPI Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");

        cy.handleUpiRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "upi_pm"
        ]["UpiCollect"];

        cy.retrievePaymentCallTest({ globalState, data });

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Refund payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "upi_pm"
        ]["Refund"];

        cy.refundCallTest(fixtures.refundBody, data, globalState);
      });
    });
  });

  // Skipping UPI Intent intentionally as connector is throwing 5xx during redirection
  context.skip("[Payment] [UPI - UPI Intent] Create & Confirm", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create payment intent -> List Merchant payment methods -> Confirm payment -> Handle UPI Redirection -> Retrieve payment", () => {
      let shouldContinue = true;

      cy.step("Create payment intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "upi_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("List Merchant payment methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Merchant payment methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "upi_pm"
        ]["UpiIntent"];

        cy.confirmUpiCall(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle UPI Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle UPI Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");

        cy.handleUpiRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });

      cy.step("Retrieve payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "upi_pm"
        ]["UpiIntent"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });
});

// TODO: This test is incomplete. Above has to be replicated here with changes to support SCL
describe.skip("UPI Payments -- Hyperswitch Stripe Compatibility Layer", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });
});
