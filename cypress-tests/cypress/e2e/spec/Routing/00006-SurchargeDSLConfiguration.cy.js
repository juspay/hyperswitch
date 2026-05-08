import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

describe("Surcharge DSL Configuration Test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        return globalState;
      })
      .then(() => {
        // Populate profileId and MCA info via prerequisite call
        cy.ListMcaByMid(globalState);
      });
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
        merchantSurchargeConfigs: {},
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
        merchantSurchargeConfigs: {},
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
        merchantSurchargeConfigs: {
          showSurchargeBreakupScreen: true,
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
