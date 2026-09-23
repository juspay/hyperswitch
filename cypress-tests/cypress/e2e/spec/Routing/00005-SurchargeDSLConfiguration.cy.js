import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

describe("Surcharge DSL Configuration Test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      if (!(globalState.get("email") && globalState.get("password"))) {
        return;
      }

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
            `[SurchargeDSLConfiguration] Login failed (${signinResp.status}): ${JSON.stringify(signinResp.body)}`
          );
        }
        const { token, token_type } = signinResp.body;
        if (token_type === "user_info") {
          globalState.set("userInfoToken", token);
        } else if (token_type === "totp") {
          cy.request({
            method: "GET",
            url: `${globalState.get("baseUrl")}/user/2fa/terminate?skip_two_factor_auth=true`,
            headers: {
              Authorization: `Bearer ${token}`,
              "Content-Type": "application/json",
            },
            failOnStatusCode: false,
          }).then((totpResp) => {
            if (totpResp.status !== 200) {
              throw new Error(
                `[SurchargeDSLConfiguration] 2FA terminate failed (${totpResp.status}): ${JSON.stringify(totpResp.body)}`
              );
            }
            globalState.set("userInfoToken", totpResp.body.token);
          });
        } else {
          throw new Error(
            `[SurchargeDSLConfiguration] Unexpected token_type "${token_type}" from signin`
          );
        }
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
          "Retrieve"
        ];

      cy.retrieveSurchargeDSLConfig(data, globalState);
    });
  });

  context("Surcharge DSL with fixed amount default selection", () => {
    it("create-surcharge-dsl-config-fixed", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "Create"
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
          "Retrieve"
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
          "Create"
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
          "Retrieve"
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
