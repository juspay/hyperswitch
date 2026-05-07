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
        merchant_surcharge_configs: {},
        algorithm: {
          type: "rate",
          rate: 2.5,
          defaultSelection: {
            surcharge_type: "rate",
            rate: 2.5,
            metadata: {},
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
          type: "fixed",
          amount: 100,
          defaultSelection: {
            surcharge_type: "fixed",
            amount: 100,
            metadata: {},
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
          type: "conditional",
          defaultSelection: {
            surcharge_type: "fixed",
            amount: 50,
            metadata: {},
          },
          rules: [
            {
              name: "Card Rule",
              connectorSelection: {
                surcharge_type: "rate",
                rate: 3.0,
              },
              conditions: [
                {
                  field: "payment_method_type",
                  operator: "equals",
                  value: "card",
                },
                {
                  field: "card_network",
                  operator: "in",
                  value: ["visa", "mastercard"],
                },
              ],
              action: {
                surcharge_type: "rate",
                rate: 3.0,
                metadata: {},
              },
            },
            {
              name: "PayPal Rule",
              connectorSelection: {
                surcharge_type: "fixed",
                amount: 200,
              },
              conditions: [
                {
                  field: "payment_method_type",
                  operator: "equals",
                  value: "paypal",
                },
              ],
              action: {
                surcharge_type: "fixed",
                amount: 200,
                metadata: {},
              },
            },
          ],
          metadata: {
            description:
              "Complex surcharge with payment method and card network conditions",
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
