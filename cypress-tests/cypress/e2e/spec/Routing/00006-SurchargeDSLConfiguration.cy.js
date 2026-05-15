import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

describe("Surcharge DSL Configuration Test", () => {
  before("seed global state and authenticate", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      const baseUrl = globalState.get("baseUrl");
      const adminApiKey = globalState.get("adminApiKey");

      const userEmail = "prajwal.nl+1@cypresstest.in";
      const userPassword = "Cypress@2025";

      cy.request({
        method: "POST",
        url: `${baseUrl}/user/signup_with_merchant_id`,
        headers: {
          "Content-Type": "application/json",
          "api-key": adminApiKey,
        },
        body: {
          email: userEmail,
          password: userPassword,
          company_name: "Juspay",
          name: "Prajwal",
        },
        failOnStatusCode: false,
      }).then((_signupResponse) => {
        cy.request({
          method: "POST",
          url: `${baseUrl}/user/v2/signin?token_only=true`,
          headers: {
            "Content-Type": "application/json",
          },
          body: {
            email: userEmail,
            password: userPassword,
          },
          failOnStatusCode: false,
        }).then((signinResponse) => {
          if (signinResponse.body.token_type === "totp") {
            const totpToken = signinResponse.body.token;

            cy.request({
              method: "GET",
              url: `${baseUrl}/user/2fa/terminate?skip_two_factor_auth=true`,
              headers: {
                Authorization: `Bearer ${totpToken}`,
                "Content-Type": "application/json",
              },
              failOnStatusCode: false,
            }).then((terminateResponse) => {
              if (terminateResponse.body.token_type === "user_info") {
                globalState.set("userInfoToken", terminateResponse.body.token);
              }
            });
          }
        });
      });
    });
  });

  before("populate profile and MCA info", () => {
    cy.ListMcaByMid(globalState);
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Surcharge DSL with rate-based default selection", () => {
    it("create-surcharge-dsl-config-rate", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "CreateRate"
        ];
      const surchargeBody = {
        name: "surcharge_config_rate",
        merchant_surcharge_configs: {},
        algorithm: {
          defaultSelection: {
            surchargeDetails: {
              surcharge: {
                type: "rate",
                value: {
                  percentage: 2.5,
                },
              },
            },
          },
          rules: [],
          metadata: {},
        },
      };

      cy.createSurchargeDSLConfig(surchargeBody, data, globalState);
    });

    it("retrieve-surcharge-dsl-config-rate", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "RetrieveRate"
        ];

      cy.retrieveSurchargeDSLConfig(data, globalState);
    });

    it("delete-surcharge-dsl-config-rate", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "Delete"
        ];

      cy.deleteSurchargeDSLConfig(data, globalState);
    });

    it("verify-delete-by-retrieve-empty", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "RetrieveDeleted"
        ];

      cy.verifySurchargeDSLConfigDeleted(data, globalState);
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
            surchargeDetails: {
              surcharge: {
                type: "fixed",
                value: {
                  amount: 100,
                },
              },
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
          "CreateConditional"
        ];
      const surchargeBody = {
        name: "surcharge_config_complex",
        merchant_surcharge_configs: {
          show_surcharge_breakup_screen: true,
        },
        algorithm: {
          defaultSelection: {
            surchargeDetails: {
              surcharge: {
                type: "rate",
                value: {
                  percentage: 2.5,
                },
              },
            },
          },
          rules: [
            {
              name: "Card Rule",
              connectorSelection: {
                surchargeDetails: {
                  surcharge: {
                    type: "rate",
                    value: {
                      percentage: 3.0,
                    },
                  },
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
                  nested: null,
                },
              ],
            },
            {
              name: "PayPal Rule",
              connectorSelection: {
                surchargeDetails: {
                  surcharge: {
                    type: "fixed",
                    value: {
                      amount: 200,
                    },
                  },
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
                        value: "paypal",
                      },
                      metadata: {},
                    },
                  ],
                  nested: null,
                },
              ],
            },
          ],
          metadata: {
            description: "Complex surcharge with payment method conditions",
          },
        },
      };

      cy.createSurchargeDSLConfig(surchargeBody, data, globalState);
    });

    it("retrieve-surcharge-dsl-config-with-rules", () => {
      const data =
        utils.getConnectorDetails("common")["SurchargeDecisionManager"][
          "RetrieveConditional"
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
