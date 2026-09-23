import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

// AuthToken JWT payload includes merchant_id and profile_id — decode locally
// so we can retarget the test at the merchant we just created.
function decodeJwtPayload(token) {
  const parts = token.split(".");
  if (parts.length !== 3) {
    throw new Error("[SurchargeDSLConfiguration] Invalid JWT format");
  }
  const b64 = parts[1].replace(/-/g, "+").replace(/_/g, "/");
  const padded = b64 + "=".repeat((4 - (b64.length % 4)) % 4);
  return JSON.parse(atob(padded));
}

describe("Surcharge DSL Configuration Test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      // Create a fresh user + merchant so we get an active AuthToken.
      // Env-based credentials don't reliably yield an AuthToken because the
      // env user may not have an active role on the test merchant.
      const uniqueSuffix = `${Date.now()}${Math.floor(Math.random() * 10000)}`;
      const surchargeEmail = `cypress_surcharge_dsl_${uniqueSuffix}@cypresstest.in`;
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
          company_name: `CypressSurchargeDSL${uniqueSuffix}`,
          name: "CypressSurchargeDSL",
        },
        failOnStatusCode: false,
      }).then((signupResp) => {
        if (signupResp.status !== 200) {
          throw new Error(
            `[SurchargeDSLConfiguration] signup_with_merchant_id failed (${signupResp.status}): ${JSON.stringify(signupResp.body)}`
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
              `[SurchargeDSLConfiguration] Signin failed (${signinResp.status}): ${JSON.stringify(signinResp.body)}`
            );
          }
          if (signinResp.body.token_type !== "totp") {
            throw new Error(
              `[SurchargeDSLConfiguration] Expected totp from signin, got "${signinResp.body.token_type}"`
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
                `[SurchargeDSLConfiguration] 2FA terminate failed (${totpResp.status}): ${JSON.stringify(totpResp.body)}`
              );
            }
            if (totpResp.body.token_type !== "user_info") {
              throw new Error(
                `[SurchargeDSLConfiguration] Expected user_info from 2FA terminate, got "${totpResp.body.token_type}"`
              );
            }
            const authToken = totpResp.body.token;
            const payload = decodeJwtPayload(authToken);
            if (!payload.merchant_id || !payload.profile_id) {
              throw new Error(
                `[SurchargeDSLConfiguration] AuthToken missing merchant_id/profile_id: ${JSON.stringify(payload)}`
              );
            }
            // Retarget the entire spec at the freshly-created merchant so the
            // surcharge DSL config is created/read/deleted on a profile we
            // actually have an active role on.
            globalState.set("userInfoToken", authToken);
            globalState.set("merchantId", payload.merchant_id);
            globalState.set("profileId", payload.profile_id);
          });
        });
      });
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Surcharge DSL with rate-based default selection", () => {
    it("create-surcharge-dsl-config-rate", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "Create"
        ];
      const surchargeBody = {
        name: "surcharge_config_rate",
        merchant_surcharge_configs: {},
        algorithm: {
          defaultSelection: {
            surcharge_details: {
              surcharge: { type: "rate", value: { percentage: 2.5 } },
            },
          },
          rules: [],
          metadata: {},
        },
      };

      cy.createSurchargeDSLConfig(surchargeBody, data, globalState);
    });

    it("retrieve-surcharge-dsl-config", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "Retrieve"
        ];

      cy.retrieveSurchargeDSLConfig(data, globalState);
    });

    it("delete-surcharge-dsl-config", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "Delete"
        ];

      cy.deleteSurchargeDSLConfig(data, globalState);
    });

    it("verify-delete-by-retrieve-empty", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "RetrieveAfterDelete"
        ];

      cy.retrieveSurchargeDSLConfig(data, globalState);
    });
  });

  context("Surcharge DSL with fixed amount default selection", () => {
    it("create-surcharge-dsl-config-fixed", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "CreateFixed"
        ];
      const surchargeBody = {
        name: "surcharge_config_fixed",
        merchant_surcharge_configs: {},
        algorithm: {
          defaultSelection: {
            surcharge_details: {
              surcharge: { type: "fixed", value: { amount: 100 } },
            },
          },
          rules: [],
          metadata: {},
        },
      };

      cy.createSurchargeDSLConfig(surchargeBody, data, globalState);
    });

    it("retrieve-surcharge-dsl-config-fixed", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "RetrieveFixed"
        ];

      cy.retrieveSurchargeDSLConfig(data, globalState);
    });

    it("delete-surcharge-dsl-config-fixed", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "Delete"
        ];

      cy.deleteSurchargeDSLConfig(data, globalState);
    });
  });

  context("Surcharge DSL with conditional rules", () => {
    it("create-surcharge-dsl-config-with-rules", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "CreateWithRules"
        ];
      const surchargeBody = {
        name: "surcharge_config_rules",
        merchant_surcharge_configs: {},
        algorithm: {
          defaultSelection: {
            surcharge_details: {
              surcharge: { type: "rate", value: { percentage: 2.5 } },
            },
          },
          rules: [
            {
              name: "card_surcharge_rule",
              connectorSelection: {
                type: "priority",
                data: [
                  {
                    connector: globalState.get("connectorId"),
                    merchant_connector_id: globalState.get(
                      `${globalState.get("connectorId")}McaId`
                    ),
                  },
                ],
              },
              surcharge_value: {
                surcharge_details: {
                  surcharge: { type: "rate", value: { percentage: 3.0 } },
                },
              },
              statements: [
                {
                  condition: [
                    {
                      lhs: "payment_method",
                      comparison: "equal",
                      value: {
                        type: "enum_variant",
                        value: "card",
                      },
                      metadata: {},
                    },
                  ],
                },
              ],
            },
          ],
          metadata: {},
        },
      };

      cy.createSurchargeDSLConfig(surchargeBody, data, globalState);
    });

    it("retrieve-surcharge-dsl-config-with-rules", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "RetrieveWithRules"
        ];

      cy.retrieveSurchargeDSLConfig(data, globalState);
    });

    it("delete-surcharge-dsl-config-with-rules", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "Delete"
        ];

      cy.deleteSurchargeDSLConfig(data, globalState);
    });
  });
});
