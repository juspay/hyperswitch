import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import * as routingUtils from "../../configs/Routing/Utils";

let globalState;

// AuthToken JWT payload includes merchant_id and profile_id — decode locally
// so we can retarget the test at the merchant we just created.
function decodeJwtPayload(token) {
  const parts = token.split(".");
  if (parts.length !== 3) {
    throw new Error(`[Surcharge] Invalid JWT format`);
  }
  const b64 = parts[1].replace(/-/g, "+").replace(/_/g, "/");
  const padded = b64 + "=".repeat((4 - (b64.length % 4)) % 4);
  return JSON.parse(atob(padded));
}

describe("Surcharge payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Surcharge payment flow test Create and confirm", () => {
    let shouldContinue = true;

    before("setup surcharge DSL", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
        if (
          utils.shouldIncludeConnector(
            globalState.get("connectorId"),
            utils.CONNECTOR_LISTS.INCLUDE.SURCHARGE
          )
        ) {
          shouldContinue = false;
          return;
        }

        // Create a fresh user + merchant so we get an active AuthToken.
        // Env-based credentials don't reliably yield an AuthToken because the
        // env user may not have an active role on the test merchant.
        const uniqueSuffix = `${Date.now()}${Math.floor(Math.random() * 10000)}`;
        const surchargeEmail = `cypress_surcharge_${uniqueSuffix}@cypresstest.in`;
        const surchargePassword = `Cypress@${uniqueSuffix}`;

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/user/signup_with_merchant_id`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("adminApiKey"),
          },
          body: {
            email: surchargeEmail,
            password: surchargePassword,
            company_name: "Juspay",
            name: "CypressSurcharge",
          },
          failOnStatusCode: false,
        }).then((signupResp) => {
          if (signupResp.status !== 200) {
            throw new Error(
              `[Surcharge] signup_with_merchant_id failed (${signupResp.status}): ${JSON.stringify(signupResp.body)}`
            );
          }

          cy.request({
            method: "POST",
            url: `${globalState.get("baseUrl")}/user/v2/signin?token_only=true`,
            headers: { "Content-Type": "application/json" },
            body: { email: surchargeEmail, password: surchargePassword },
            failOnStatusCode: false,
          }).then((signinResp) => {
            if (signinResp.status !== 200) {
              throw new Error(
                `[Surcharge] Signin failed (${signinResp.status}): ${JSON.stringify(signinResp.body)}`
              );
            }
            if (signinResp.body.token_type !== "totp") {
              throw new Error(
                `[Surcharge] Expected totp from signin, got "${signinResp.body.token_type}"`
              );
            }

            cy.request({
              method: "GET",
              url: `${globalState.get("baseUrl")}/user/2fa/terminate?skip_two_factor_auth=true`,
              headers: {
                Authorization: `Bearer ${signinResp.body.token}`,
                "Content-Type": "application/json",
              },
              failOnStatusCode: false,
            }).then((totpResp) => {
              if (totpResp.status !== 200) {
                throw new Error(
                  `[Surcharge] 2FA terminate failed (${totpResp.status}): ${JSON.stringify(totpResp.body)}`
                );
              }
              if (totpResp.body.token_type !== "user_info") {
                throw new Error(
                  `[Surcharge] Expected user_info from 2FA terminate, got "${totpResp.body.token_type}"`
                );
              }
              const authToken = totpResp.body.token;
              const payload = decodeJwtPayload(authToken);
              if (!payload.merchant_id || !payload.profile_id) {
                throw new Error(
                  `[Surcharge] AuthToken missing merchant_id/profile_id: ${JSON.stringify(payload)}`
                );
              }
              // Retarget the entire test at the freshly-created merchant so the
              // surcharge DSL and payment share a merchant.
              globalState.set("userInfoToken", authToken);
              globalState.set("merchantId", payload.merchant_id);
              globalState.set("profileId", payload.profile_id);
            });
          });
        });

        // Bootstrap the fresh merchant with an api-key, a customer, and the
        // authorizedotnet connector so the payment flow can run against it.
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState
        );

        const dslData =
          routingUtils.getConnectorDetails("common")[
            "SurchargeDecisionManager"
          ]["Create"];
        cy.createSurchargeDSLConfig(dslData.Request, dslData, globalState);
      });
    });

    after("cleanup surcharge DSL", () => {
      if (shouldContinue) {
        const dslData =
          routingUtils.getConnectorDetails("common")[
            "SurchargeDecisionManager"
          ]["Delete"];
        cy.deleteSurchargeDSLConfig(dslData, globalState);
      }
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment -> Retrieve Payment", () => {
      let continueSteps = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SurchargeDSL"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          continueSteps = false;
        }
      });

      cy.step("Payment Methods Call", () => {
        if (!continueSteps) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!continueSteps) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SurchargeDSLConfirm"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          continueSteps = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!continueSteps) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SurchargeDSLConfirm"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });
});
