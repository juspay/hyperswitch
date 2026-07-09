import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import * as routingUtils from "../../configs/Routing/Utils";

let globalState;

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
        if (globalState.get("email") && globalState.get("password")) {
          cy.request({
            method: "POST",
            url: `${globalState.get("baseUrl")}/user/v2/signin?token_only=true`,
            headers: { "Content-Type": "application/json" },
            body: {
              email: globalState.get("email"),
              password: globalState.get("password"),
            },
            failOnStatusCode: false,
          }).then((signinResp) => {
            if (signinResp.status !== 200) {
              throw new Error(
                `[Surcharge] Login failed (${signinResp.status}): ${JSON.stringify(signinResp.body)}`
              );
            }
            const { token: signinToken, token_type: signinType } =
              signinResp.body;

            const resolveAuthToken = (bearerToken, tokenType) => {
              if (tokenType === "user_info") {
                globalState.set("userInfoToken", bearerToken);
              } else if (tokenType === "accept_invite") {
                // User has no active merchant role — accept invitation to activate it
                cy.request({
                  method: "POST",
                  url: `${globalState.get("baseUrl")}/user/invite/accept/pre_auth`,
                  headers: {
                    Authorization: `Bearer ${bearerToken}`,
                    "Content-Type": "application/json",
                  },
                  body: [
                    {
                      entity_id: globalState.get("merchantId"),
                      entity_type: "merchant",
                    },
                  ],
                  failOnStatusCode: false,
                }).then((acceptResp) => {
                  if (acceptResp.status !== 200) {
                    throw new Error(
                      `[Surcharge] Accept invitation failed (${acceptResp.status}): ${JSON.stringify(acceptResp.body)}`
                    );
                  }
                  if (acceptResp.body.token_type === "user_info") {
                    globalState.set("userInfoToken", acceptResp.body.token);
                  } else {
                    throw new Error(
                      `[Surcharge] Unexpected token_type "${acceptResp.body.token_type}" after accepting invitation`
                    );
                  }
                });
              } else {
                throw new Error(
                  `[Surcharge] Unexpected token_type "${tokenType}" — cannot obtain an AuthToken`
                );
              }
            };

            if (signinType === "user_info") {
              resolveAuthToken(signinToken, "user_info");
            } else if (signinType === "totp") {
              cy.request({
                method: "GET",
                url: `${globalState.get("baseUrl")}/user/2fa/terminate?skip_two_factor_auth=true`,
                headers: {
                  Authorization: `Bearer ${signinToken}`,
                  "Content-Type": "application/json",
                },
                failOnStatusCode: false,
              }).then((totpResp) => {
                if (totpResp.status !== 200) {
                  throw new Error(
                    `[Surcharge] 2FA terminate failed (${totpResp.status}): ${JSON.stringify(totpResp.body)}`
                  );
                }
                resolveAuthToken(totpResp.body.token, totpResp.body.token_type);
              });
            } else {
              throw new Error(
                `[Surcharge] Unexpected token_type "${signinType}" from signin`
              );
            }
          });
        }
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
