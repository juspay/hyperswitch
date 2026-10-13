import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

describe("Surcharge DSL Configuration Test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      // Create a fresh user + merchant so we get an active AuthToken.
      // Env-based credentials don't reliably yield an AuthToken because the
      // env user may not have an active role on the test merchant.
      cy.signupUserWithMerchant("CypressSurchargeDSL", globalState);

      // Login sequence used elsewhere in the suite: userLogin sets
      // totpToken, terminate2Fa exchanges it for userInfoToken, and
      // userInfo reads merchantId/organizationId/profileId off /user —
      // retargeting the spec at the merchant we just created.
      cy.userLogin(globalState);
      cy.terminate2Fa(globalState);
      cy.userInfo(globalState);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  after("cleanup throwaway merchant", () => {
    if (globalState.get("merchantId")) {
      cy.merchantDeleteCall(globalState);
    }
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
