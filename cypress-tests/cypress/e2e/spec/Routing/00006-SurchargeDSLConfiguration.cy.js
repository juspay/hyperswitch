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

  beforeEach(function () {
    // Surcharge DSL endpoints require JWT auth in release builds (integ/sandbox).
    // Skip these tests on environments where api-key auth is not accepted.
    const baseUrl = globalState.get("baseUrl") || "";
    if (
      baseUrl.includes("integ") ||
      baseUrl.includes("sandbox") ||
      baseUrl.includes("prod")
    ) {
      cy.log(
        "SKIPPED: Surcharge DSL tests require JWT authentication on this environment (release build)."
      );
      this.skip();
    }
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
            surcharge_details: {
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
            surcharge_details: {
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
            surcharge_details: {
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
                surcharge_details: {
                  surcharge: {
                    type: "rate",
                    value: {
                      percentage: 3.0,
                    },
                  },
                  tax_on_surcharge: null,
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
              name: "Pay Later Rule",
              connectorSelection: {
                surcharge_details: {
                  surcharge: {
                    type: "fixed",
                    value: {
                      amount: 200,
                    },
                  },
                  tax_on_surcharge: null,
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
                        value: "pay_later",
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
