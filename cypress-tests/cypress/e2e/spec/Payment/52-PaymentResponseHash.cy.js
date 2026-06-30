import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import { setup3DSPayment } from "../../../utils/paymentHelpers";

let globalState;

describe("Card - Payment Response Hash flow test", () => {
  // Gate variable: set to false when the connector does not support
  // payment-response-hash or when the merchant has the feature disabled.
  // Used in beforeEach to skip every test in this describe block.
  let shouldContinue = true;

  before("seed global state and check account config", function () {
    return cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      // Check if the current connector supports the payment response hash feature
      // This gate ensures the test only runs for connectors explicitly configured
      // with PAYMENT_RESPONSE_HASH support (currently only Stripe)
      if (
        !utils.CONNECTOR_LISTS.INCLUDE.PAYMENT_RESPONSE_HASH.includes(
          globalState.get("connectorId")
        )
      ) {
        shouldContinue = false;
        return;
      }
      return cy.fetchPaymentResponseHashConfig(globalState).then(() => {
        const enablePaymentResponseHash = globalState.get(
          "enablePaymentResponseHash"
        );
        if (!enablePaymentResponseHash) {
          cy.task(
            "cli_log",
            "enable_payment_response_hash is false - skipping spec"
          );
          shouldContinue = false;
        }
      });
    });
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("No3DS Auto-Capture - Verify Payment Response Hash Config", () => {
    it("create payment intent -> confirm payment -> verify payment response hash", () => {
      // Per-step gate variable: tracks whether the preceding API call
      // succeeded so that subsequent steps can be skipped gracefully when
      // a connector does not support the No3DS auto-capture flow.
      let stepContinue = true;

      cy.step("create payment intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        // Create a payment intent with no-3DS authentication and automatic capture
        // "no_three_ds": disables 3-D Secure authentication for a frictionless card flow
        // "automatic": captures the authorized amount immediately without manual capture step
        // globalState: shared state object for persisting payment data across test steps
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds", // authentication type: no 3-D Secure challenge required
          "automatic", // capture method: auto-capture (immediate settlement)
          globalState // shared mutable state for cross-step data persistence
        );

        if (!utils.should_continue_further(data)) {
          stepContinue = false;
        }
      });

      cy.step("confirm payment", () => {
        if (!stepContinue) {
          cy.task("cli_log", "Skipping step: confirm payment");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          stepContinue = false;
        }
      });

      cy.step("retrieve payment", () => {
        if (!stepContinue) {
          cy.task("cli_log", "Skipping step: retrieve payment");
          return;
        }

        cy.retrievePaymentCallTest({ globalState });
      });

      cy.step("verify payment response hash", () => {
        if (!stepContinue) {
          cy.task("cli_log", "Skipping step: verify payment response hash");
          return;
        }

        // Verify payment response hash is enabled in business profile config
        cy.assertPaymentResponseHashEnabled(globalState);
      });
    });
  });

  context("3DS Auto-Capture - Verify Redirect Signature", () => {
    it("setup 3DS -> verify redirect signature", () => {
      // Setup 3DS payment flow without redirection to test signature verification
      // globalState: shared state object for test data persistence
      // includeRedirection: false - skips the actual browser redirection to focus on signature validation
      // fixtures: test data fixtures for payment setup
      setup3DSPayment(globalState, { includeRedirection: false, fixtures });

      cy.step("verify redirect signature", () => {
        // Assert that redirect URL was captured before attempting signature verification
        // This prevents silent passes when captureRedirectReturnUrl fails
        expect(
          globalState.get("redirectReturnUrl"),
          "redirectReturnUrl must be captured from 3DS setup"
        ).to.exist.and.not.be.empty;

        cy.verifyRedirectSignature(globalState);
      });
    });
  });

  context("3DS Auto-Capture - Compute and Verify Redirect Signature", () => {
    it("setup 3DS -> compute HMAC and compare with redirect signature", () => {
      // Setup 3DS payment flow to test HMAC computation and signature comparison
      // globalState: shared state object for test data persistence
      // includeRedirection: false - skips browser redirection to focus on HMAC computation logic
      // fixtures: test data fixtures for payment setup
      setup3DSPayment(globalState, { includeRedirection: false, fixtures });

      cy.step("compute and verify redirect signature", () => {
        if (!globalState.get("_setup3DSContinue")) {
          cy.task(
            "cli_log",
            "Skipping step: compute and verify redirect signature"
          );
          return;
        }

        cy.computeAndVerifyRedirectSignature(globalState);
      });
    });
  });

  context("3DS Auto-Capture - Failure Scenarios for Invalid Signatures", () => {
    it("setup 3DS -> compute HMAC -> verify tampered and wrong-key signatures fail", () => {
      // Setup 3DS payment flow for testing tampered and wrong-key signature scenarios
      // globalState: shared state object for test data persistence
      // includeRedirection: false - skips browser redirection to test signature validation failures
      // fixtures: test data fixtures for payment setup
      setup3DSPayment(globalState, { includeRedirection: false, fixtures });

      cy.step("compute and verify redirect signature", () => {
        if (!globalState.get("_setup3DSContinue")) {
          cy.task(
            "cli_log",
            "Skipping step: compute and verify redirect signature"
          );
          return;
        }

        cy.computeAndVerifyRedirectSignature(globalState);
      });

      cy.step("verify tampered signature fails", () => {
        if (!globalState.get("_setup3DSContinue")) {
          cy.task("cli_log", "Skipping step: verify tampered signature fails");
          return;
        }

        cy.verifyTamperedSignatureFails(globalState);
      });

      cy.step("verify wrong-key signature fails", () => {
        if (!globalState.get("_setup3DSContinue")) {
          cy.task("cli_log", "Skipping step: verify wrong-key signature fails");
          return;
        }

        cy.verifyWrongKeySignatureFails(globalState);
      });
    });
  });
});
