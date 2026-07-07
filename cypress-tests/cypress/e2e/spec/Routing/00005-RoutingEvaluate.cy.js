import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

describe("Routing Evaluate Test", () => {
  context("Evaluate routing: card payment method rule → stripe", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("retrieve-mca", () => {
      cy.ListMcaByMid(globalState);
    });

    it("add-routing-config", () => {
      const data = utils.getConnectorDetails("common")["routingEvaluate"];
      const routing_data = {
        defaultSelection: {
          type: "priority",
          data: [],
        },
        metadata: {},
        rules: [
          {
            name: "rule_card_to_stripe",
            connectorSelection: {
              type: "priority",
              data: [
                {
                  connector: "stripe",
                  merchant_connector_id: globalState.get("stripeMcaId"),
                },
              ],
            },
            statements: [
              {
                condition: [
                  {
                    lhs: "payment_method",
                    comparison: "equal",
                    value: { type: "enum_variant", value: "card" },
                    metadata: {},
                  },
                ],
              },
            ],
          },
        ],
      };

      cy.addRoutingConfig(
        fixtures.routingConfigBody,
        data,
        "advanced",
        routing_data,
        globalState
      );
    });

    it("retrieve-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["routingEvaluate"];
      cy.retrieveRoutingConfig(data, globalState);
    });

    it("activate-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["routingEvaluate"];
      cy.activateRoutingConfig(data, globalState);
    });

    it("evaluate-routing-card-to-stripe", () => {
      const data = utils.getConnectorDetails("common")["routingEvaluate"];

      const evaluate_params = {
        expectedConnector: "stripe",
        parameters: {
          payment_method: { type: "enum_variant", value: "card" },
          amount: { type: "number", value: 100 },
          currency: { type: "enum_variant", value: "USD" },
        },
        fallback_output: [
          {
            gateway_name: "stripe",
            gateway_id: globalState.get("stripeMcaId"),
          },
        ],
      };

      cy.evaluateRoutingRule(data, evaluate_params, globalState);
    });
  });

  context(
    "Evaluate routing: currency-based rule (USD → stripe, default → adyen)",
    () => {
      before("seed global state", () => {
        cy.task("getGlobalState").then((state) => {
          globalState = new State(state);
        });
      });

      afterEach("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("retrieve-mca", () => {
        cy.ListMcaByMid(globalState);
      });

      it("add-currency-routing-config", () => {
        const data = utils.getConnectorDetails("common")["routingEvaluate"];
        const routing_data = {
          defaultSelection: {
            type: "priority",
            data: [
              {
                connector: "adyen",
                merchant_connector_id: globalState.get("adyenMcaId"),
              },
            ],
          },
          metadata: {},
          rules: [
            {
              name: "rule_usd_to_stripe",
              connectorSelection: {
                type: "priority",
                data: [
                  {
                    connector: "stripe",
                    merchant_connector_id: globalState.get("stripeMcaId"),
                  },
                ],
              },
              statements: [
                {
                  condition: [
                    {
                      lhs: "currency",
                      comparison: "equal",
                      value: { type: "enum_variant", value: "USD" },
                      metadata: {},
                    },
                  ],
                },
              ],
            },
          ],
        };

        cy.addRoutingConfig(
          fixtures.routingConfigBody,
          data,
          "advanced",
          routing_data,
          globalState
        );
      });

      it("retrieve-routing-call-test", () => {
        const data = utils.getConnectorDetails("common")["routingEvaluate"];
        cy.retrieveRoutingConfig(data, globalState);
      });

      it("activate-routing-call-test", () => {
        const data = utils.getConnectorDetails("common")["routingEvaluate"];
        cy.activateRoutingConfig(data, globalState);
      });

      it("evaluate-routing-usd-to-stripe", () => {
        const data = utils.getConnectorDetails("common")["routingEvaluate"];

        const evaluate_params = {
          expectedConnector: "stripe",
          parameters: {
            currency: { type: "enum_variant", value: "USD" },
            amount: { type: "number", value: 100 },
          },
          fallback_output: [],
        };

        cy.evaluateRoutingRule(data, evaluate_params, globalState);
      });
    }
  );
});
