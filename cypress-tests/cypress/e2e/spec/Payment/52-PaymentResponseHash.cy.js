import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Payment Response Hash flow test", () => {
  before("seed global state and check account config", function () {
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
      })
      .then(() => {
        cy.fetchPaymentResponseHashConfig(globalState);
      })
      .then((response) => {
        if (!response || response.status !== 200) {
          cy.task("cli_log", "Failed to fetch account config - skipping spec");
          this.skip();
          return;
        }

        const enablePaymentResponseHash =
          response.body.enable_payment_response_hash;
        const paymentResponseHashKey = response.body.payment_response_hash_key;

        if (!enablePaymentResponseHash) {
          cy.task(
            "cli_log",
            "enable_payment_response_hash is false/absent - skipping spec"
          );
          this.skip();
          return;
        }

        globalState.set("paymentResponseHashKey", paymentResponseHashKey);
        globalState.set("enablePaymentResponseHash", enablePaymentResponseHash);

        cy.task(
          "cli_log",
          `Account config verified - enable_payment_response_hash: true, key length: ${paymentResponseHashKey.length}`
        );
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("No3DS Auto-Capture - Verify Payment Response Hash Config", () => {
    it("create payment intent -> confirm payment -> verify payment response hash", () => {
      let shouldContinue = true;

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
          shouldContinue = false;
        }
      });

      cy.step("confirm payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: confirm payment");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("retrieve payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: retrieve payment");
          return;
        }

        cy.retrievePaymentCallTest({ globalState });
      });

      cy.step("verify payment response hash", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: verify payment response hash");
          return;
        }

        cy.assertPaymentResponseHashEnabled(globalState);
      });
    });
  });

  context("3DS Auto-Capture - Verify Redirect Signature", () => {
    it("setup 3DS -> verify redirect signature", () => {
      cy.setup3DSPayment(globalState, { includeRedirection: true });

      cy.step("verify redirect signature", () => {
        if (!globalState.get("_setup3DSContinue")) {
          cy.task("cli_log", "Skipping step: verify redirect signature");
          return;
        }

        cy.verifyRedirectSignature(globalState);
      });
    });
  });

  context("3DS Auto-Capture - Compute and Verify Redirect Signature", () => {
    it("setup 3DS -> compute HMAC and compare with redirect signature", () => {
      cy.setup3DSPayment(globalState, { includeRedirection: false });

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
      cy.setup3DSPayment(globalState, { includeRedirection: false });

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

      cy.step("verify tampered and wrong-key signatures fail", () => {
        if (!globalState.get("_setup3DSContinue")) {
          cy.task("cli_log", "Skipping step: verify tampered signatures fail");
          return;
        }

        cy.then(() => {
          const computedSignature = globalState.get("computedSignature");
          if (!computedSignature) {
            cy.task(
              "cli_log",
              "No computed signature (no redirect URL) - skipping tampered signature verification"
            );
            return;
          }
          cy.verifyTamperedSignatureFails(globalState);
        });
      });
    });
  });

  context("3DS Auto-Capture - Webhook Signature Verification", () => {
    it("setup 3DS -> verify webhook delivery signature", () => {
      cy.setup3DSPayment(globalState, { includeRedirection: true });

      cy.step("wait for webhook delivery and verify signature", () => {
        if (!globalState.get("_setup3DSContinue")) {
          cy.task("cli_log", "Skipping step: webhook signature verification");
          return;
        }

        cy.fetchWebhookWithRetry(globalState);
        cy.verifyWebhookSignatureHeader(globalState);
      });
    });
  });
});
