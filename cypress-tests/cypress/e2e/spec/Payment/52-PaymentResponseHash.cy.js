import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import { setup3DSPayment } from "../../../utils/paymentHelpers";

let globalState;

describe("Card - Payment Response Hash flow test", () => {
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
      let stepContinue = true;

      cy.step("create payment intent", () => {
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

/**
 * Negative test: when enable_payment_response_hash = false,
 * redirect URLs must NOT contain signature or signature_algorithm params.
 *
 * This runs independently of the positive tests above so it can verify
 * the absence of hash-related query params when the feature is disabled.
 */
describe("Card - Payment Response Hash Negative Test", () => {
  let negGlobalState;
  let negShouldContinue = false;

  before("seed global state and verify hash is disabled", function () {
    return cy.task("getGlobalState").then((state) => {
      negGlobalState = new State(state);
      // Only test connectors that support the feature in principle
      if (
        !utils.CONNECTOR_LISTS.INCLUDE.PAYMENT_RESPONSE_HASH.includes(
          negGlobalState.get("connectorId")
        )
      ) {
        return;
      }
      return cy.fetchPaymentResponseHashConfig(negGlobalState).then(() => {
        const enabled = negGlobalState.get("enablePaymentResponseHash");
        // Only activate the negative test when the fetch succeeded AND
        // explicitly confirmed the feature is disabled (stored as false).
        // If enabled is undefined, the fetch failed — skip rather than guess.
        if (enabled === false) {
          negShouldContinue = true;
          cy.task(
            "cli_log",
            "Negative test active: enable_payment_response_hash is explicitly false"
          );
        }
      });
    });
  });

  beforeEach(function () {
    if (!negShouldContinue) {
      this.skip();
    }
  });

  after("flush global state", () => {
    cy.task("setGlobalState", negGlobalState.data);
  });

  context("Hash Disabled - Verify Redirect URL Has No Signature Params", () => {
    it("setup 3DS -> capture redirect URL -> verify no signature params", () => {
      setup3DSPayment(negGlobalState, { includeRedirection: false, fixtures });

      cy.step("verify redirect URL has no signature params", () => {
        const redirectUrl =
          negGlobalState.get("redirectReturnUrl") ||
          negGlobalState.get("nextActionUrl");

        if (!redirectUrl) {
          cy.task(
            "cli_log",
            "No redirect URL available - skipping negative assertion"
          );
          return;
        }

        const urlObj = new URL(redirectUrl);
        const signature = urlObj.searchParams.get("signature");
        const signatureAlgorithm = urlObj.searchParams.get(
          "signature_algorithm"
        );

        expect(signature, "signature must be absent when hash is disabled").to
          .be.null;
        expect(
          signatureAlgorithm,
          "signature_algorithm must be absent when hash is disabled"
        ).to.be.null;

        cy.task(
          "cli_log",
          "Negative test PASSED: redirect URL has no signature params when hash is disabled"
        );
      });
    });
  });
});
