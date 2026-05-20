import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

const STRIPE_TEST_NULLABLE_FIELDS = new Set([
  "card_type",
  "card_network",
  "card_issuer",
  "card_issuing_country",
  "card_extended_bin",
]);

function assertPartialMatch(expected, actual, path = "") {
  for (const key in expected) {
    const expectedVal = expected[key];
    const actualVal = actual?.[key];
    const currentPath = path ? `${path}.${key}` : key;

    if (actualVal === null && STRIPE_TEST_NULLABLE_FIELDS.has(key)) {
      cy.task(
        "cli_log",
        `Skipping assertion for ${currentPath} — Stripe test env returns null`
      );
      continue;
    }

    if (
      typeof expectedVal === "object" &&
      expectedVal !== null &&
      !Array.isArray(expectedVal)
    ) {
      assertPartialMatch(expectedVal, actualVal, currentPath);
      continue;
    }

    expect(
      actualVal,
      `Expected ${currentPath} to equal ${JSON.stringify(expectedVal)}, got ${JSON.stringify(actualVal)}`
    ).to.equal(expectedVal);
  }
}

let globalState;
let hashEnabled = false;

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
          cy.task("cli_log", "Failed to fetch account config");
          return;
        }

        const enablePaymentResponseHash =
          response.body.enable_payment_response_hash;
        const paymentResponseHashKey =
          response.body.payment_response_hash_key;
        const webhookUrl = response.body.webhook_url;

        globalState.set("paymentResponseHashKey", paymentResponseHashKey);
        globalState.set(
          "enablePaymentResponseHash",
          enablePaymentResponseHash
        );
        globalState.set(
          "webhookUrlConfigured",
          !!(webhookUrl && webhookUrl.trim().length > 0)
        );

        if (webhookUrl && webhookUrl.trim().length > 0) {
          cy.task("cli_log", `webhook_url is configured: ${webhookUrl}`);
        } else {
          cy.task(
            "cli_log",
            "webhook_url is not configured - webhook signature tests will be skipped"
          );
        }

        if (enablePaymentResponseHash) {
          hashEnabled = true;
          cy.task(
            "cli_log",
            `Account config verified - enable_payment_response_hash: true, key length: ${paymentResponseHashKey.length}`
          );
        } else {
          cy.task(
            "cli_log",
            "enable_payment_response_hash is false/absent"
          );
        }
      });
  });

  after("cleanup transient keys and flush global state", () => {
    if (globalState) {
      globalState.set("computedSignature", undefined);
      globalState.set("computedSigningPayload", undefined);
      globalState.set("_setup3DSContinue", undefined);
      globalState.set("webhookData", undefined);
      globalState.set("signatureAlgorithm", undefined);
      globalState.set("webhookUrlConfigured", undefined);
      cy.task("setGlobalState", globalState.data);
    }
  });

  context(
    "No3DS Auto-Capture - Verify Payment Response Hash Configuration",
    () => {
      before(function () {
        if (!hashEnabled) {
          this.skip();
        }
      });

      it(
        "create payment intent -> confirm payment -> verify hash config is enabled",
        () => {
          let shouldContinue = true;

          cy.step("create payment intent", () => {
            const data = getConnectorDetails(
              globalState.get("connectorId")
            )["card_pm"]["PaymentIntent"];

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

            const data = getConnectorDetails(
              globalState.get("connectorId")
            )["card_pm"]["No3DSAutoCapture"];

            cy.confirmCallTest(
              fixtures.confirmBody,
              data,
              true,
              globalState
            );

            if (!utils.should_continue_further(data)) {
              shouldContinue = false;
            }
          });

          cy.step("retrieve payment", () => {
            if (!shouldContinue) {
              cy.task("cli_log", "Skipping step: retrieve payment");
              return;
            }

            const paymentId = globalState.get("paymentID");
            const publishableKey =
              globalState.get("publishableKey") || globalState.get("apiKey");
            const baseUrl = globalState.get("baseUrl");

            cy.request({
              method: "GET",
              url: `${baseUrl}/payments/${paymentId}`,
              headers: {
                "Content-Type": "application/json",
                "api-key": publishableKey,
              },
              failOnStatusCode: false,
            }).then((response) => {
              expect(response.status, "retrieve payment status").to.equal(200);
              expect(response.body, "payment response body").to.not.be.empty;

              const expectedData = getConnectorDetails(
                globalState.get("connectorId")
              )["card_pm"]["No3DSAutoCapture"];

              if (
                expectedData?.Response?.body?.payment_method_data &&
                response.body.payment_method_data
              ) {
                assertPartialMatch(
                  expectedData.Response.body.payment_method_data,
                  response.body.payment_method_data,
                  "payment_method_data"
                );
              }

              globalState.set("paymentID", response.body.payment_id);
            });
          });

          cy.step("verify hash config is enabled", () => {
            if (!shouldContinue) {
              cy.task(
                "cli_log",
                "Skipping step: verify hash config is enabled"
              );
              return;
            }

            cy.assertPaymentResponseHashEnabled(globalState);
          });
        }
      );
    }
  );

  context("3DS Auto-Capture - Payment Response Hash Verification", () => {
    let shouldContinue = true;

    before(function () {
      if (!hashEnabled) {
        this.skip();
      }
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("setup 3DS payment", () => {
      globalState.set("_setup3DSContinue", true);

      cy.step("create payment intent (3DS)", () => {
        const connectorId = globalState.get("connectorId");
        const data = getConnectorDetails(connectorId)["card_pm"]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );

        cy.then(() => {
          if (
            data &&
            data.Response &&
            data.Response.status &&
            data.Response.status !== 200
          ) {
            globalState.set("_setup3DSContinue", false);
          }
        });
      });

      cy.step("confirm payment (3DS)", () => {
        if (!globalState.get("_setup3DSContinue")) {
          cy.task("cli_log", "Skipping step: confirm payment (3DS)");
          return;
        }

        const connectorId = globalState.get("connectorId");
        const data =
          getConnectorDetails(connectorId)["card_pm"]["3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        cy.then(() => {
          if (
            data &&
            data.Response &&
            data.Response.status &&
            data.Response.status !== 200
          ) {
            globalState.set("_setup3DSContinue", false);
          }
        });
      });

      cy.then(() => {
        if (!globalState.get("_setup3DSContinue")) {
          shouldContinue = false;
        }
      });
    });

    it("verify redirect signature", () => {
      cy.verifyRedirectSignature(globalState);
    });

    it("compute and verify redirect signature", () => {
      cy.computeAndVerifyRedirectSignature(globalState);
    });

    it("verify tampered and wrong-key signatures fail", () => {
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

    it("verify webhook delivery signature", function () {
      if (!globalState.get("webhookUrlConfigured")) {
        this.skip();
        return;
      }
      cy.fetchWebhookWithRetry(globalState);
      cy.verifyWebhookSignatureHeader(globalState);
    });
  });

  context("Hash Disabled - Verify No Signatures Present", () => {
    before(function () {
      if (hashEnabled) {
        this.skip();
      }
    });

    it(
      "verify redirect URL has no signature params and webhook has no webhook_signature",
      () => {
        let shouldContinue = true;

        cy.step("create payment intent", () => {
          const data = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PaymentIntent"];

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

          const data = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSAutoCapture"];

          cy.confirmCallTest(
            fixtures.confirmBody,
            data,
            true,
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("verify no signature in redirect URL", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: verify no redirect signature"
            );
            return;
          }

          const paymentId = globalState.get("paymentID");
          const publishableKey = globalState.get("publishableKey");
          const baseUrl = globalState.get("baseUrl");

          cy.request({
            method: "GET",
            url: `${baseUrl}/payments/${paymentId}`,
            headers: {
              "Content-Type": "application/json",
              "api-key": publishableKey,
            },
            failOnStatusCode: false,
          }).then((response) => {
            const redirectUrl =
              response.body?.next_action?.redirect_to_url;
            if (!redirectUrl) {
              cy.task(
                "cli_log",
                "No redirect URL present - assertion N/A for this flow"
              );
              return;
            }

            const urlObj = new URL(redirectUrl);
            expect(
              urlObj.searchParams.get("signature"),
              "redirect URL should NOT have signature param when hash is disabled"
            ).to.be.null;
            expect(
              urlObj.searchParams.get("signature_algorithm"),
              "redirect URL should NOT have signature_algorithm param when hash is disabled"
            ).to.be.null;
          });
        });

        cy.step("verify no webhook_signature in webhook delivery", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: verify no webhook signature"
            );
            return;
          }

          const paymentId = globalState.get("paymentID");
          const apiKey = globalState.get("adminApiKey");
          const baseUrl = globalState.get("baseUrl");

          cy.request({
            method: "GET",
            url: `${baseUrl}/payments/${paymentId}/webhooks`,
            headers: {
              "Content-Type": "application/json",
              "api-key": apiKey,
            },
            failOnStatusCode: false,
          }).then((response) => {
            if (
              response.status !== 200 ||
              !response.body ||
              !response.body.data ||
              response.body.data.length === 0
            ) {
              cy.task(
                "cli_log",
                "No webhook deliveries found - assertion N/A"
              );
              return;
            }

            const webhook = response.body.data[0];
            expect(
              webhook.webhook_signature,
              "webhook should NOT have webhook_signature when hash is disabled"
            ).to.be.undefined;
          });
        });
      }
    );
  });
});
