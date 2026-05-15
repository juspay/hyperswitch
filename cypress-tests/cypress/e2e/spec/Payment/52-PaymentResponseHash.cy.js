import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Payment Response Hash flow test", () => {
  before("seed global state and probe account config", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connectorId = globalState.get("connectorId");

        if (
          utils.shouldExcludeConnector(
            connectorId,
            utils.CONNECTOR_LISTS.EXCLUDE.PAYMENT_RESPONSE_HASH
          )
        ) {
          skip = true;
          return;
        }

        const merchantId = globalState.get("merchantId");
        const apiKey = globalState.get("adminApiKey");
        const baseUrl = globalState.get("baseUrl");

        cy.request({
          method: "GET",
          url: `${baseUrl}/accounts/${merchantId}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": apiKey,
          },
          failOnStatusCode: false,
        }).then((response) => {
          const enabled = response.body?.enable_payment_response_hash;
          const hashKey = response.body?.payment_response_hash_key;

          if (!enabled) {
            cy.task(
              "cli_log",
              `enable_payment_response_hash is ${enabled} for merchant ${merchantId} — skipping spec`
            );
            skip = true;
            return;
          }

          globalState.set("enablePaymentResponseHash", enabled);
          globalState.set("paymentResponseHashKey", hashKey || "");

          cy.task(
            "cli_log",
            `Payment response hash enabled for merchant ${merchantId} — key length: ${(hashKey || "").length}`
          );
        });
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
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

        cy.verifyPaymentResponseHash(globalState);
      });
    });
  });

  context("3DS Auto-Capture - Verify Redirect Signature", () => {
    it("create payment intent -> confirm 3DS payment -> handle redirection -> verify redirect signature", () => {
      let shouldContinue = true;

      cy.step("create payment intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
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

      cy.step("payment methods call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: payment methods call");
          return;
        }

        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("confirm 3DS payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: confirm 3DS payment");
          return;
        }

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];

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

      cy.step("handle redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: handle redirection");
          return;
        }

        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      cy.step("verify redirect signature", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: verify redirect signature");
          return;
        }

        cy.verifyRedirectSignature(globalState);
      });
    });
  });

  context("3DS Auto-Capture - Compute and Verify HMAC-SHA512 Signature", () => {
    it("create payment intent -> confirm 3DS -> compute HMAC-SHA512 and compare with redirect signature", () => {
      let shouldContinue = true;

      cy.step("create payment intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
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

      cy.step("payment methods call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: payment methods call");
          return;
        }

        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("confirm 3DS payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: confirm 3DS payment");
          return;
        }

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];

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

      cy.step("compute and verify HMAC-SHA512 signature", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: compute and verify HMAC-SHA512");
          return;
        }

        cy.computeAndVerifyRedirectSignature(globalState);
      });
    });
  });

  context("3DS Auto-Capture - Failure Scenarios for Invalid Signatures", () => {
    it("create payment intent -> confirm 3DS -> compute HMAC -> verify tampered, wrong-key, wrong-algorithm signatures fail", () => {
      let shouldContinue = true;

      cy.step("create payment intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
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

      cy.step("payment methods call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: payment methods call");
          return;
        }

        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("confirm 3DS payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: confirm 3DS payment");
          return;
        }

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];

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

      cy.step("compute and verify HMAC-SHA512 signature", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: compute and verify HMAC-SHA512");
          return;
        }

        cy.computeAndVerifyRedirectSignature(globalState);
      });

      cy.step(
        "verify tampered, wrong-key, and wrong-algorithm signatures fail",
        () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: failure scenarios");
            return;
          }

          cy.verifyTamperedSignatureFails(globalState);
        }
      );
    });
  });

  context("3DS Auto-Capture - Webhook Signature Verification", () => {
    it("create payment intent -> confirm 3DS -> verify webhook delivery signature", () => {
      let shouldContinue = true;

      cy.step("create payment intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
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

      cy.step("payment methods call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: payment methods call");
          return;
        }

        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("confirm 3DS payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: confirm 3DS payment");
          return;
        }

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];

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

      cy.step("wait for webhook delivery and verify signature", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: webhook signature verification");
          return;
        }

        cy.wait(5000);

        cy.verifyWebhookSignatureHeader(globalState);
      });
    });
  });
});
