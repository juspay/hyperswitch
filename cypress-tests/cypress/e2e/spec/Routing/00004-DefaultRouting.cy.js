import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

describe("Default Routing Test", () => {
  context("Default priority routing (stripe first)", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("list-mca-by-mid", () => {
      cy.ListMcaByMid(globalState);
    });

    it("add-routing-config", () => {
      const data = utils.getConnectorDetails("common")["defaultRouting"];
      const routing_data = [
        {
          connector: "stripe",
          merchant_connector_id: globalState.get("stripeMcaId"),
        },
        {
          connector: "adyen",
          merchant_connector_id: globalState.get("adyenMcaId"),
        },
      ];
      cy.addRoutingConfig(
        fixtures.routingConfigBody,
        data,
        "priority",
        routing_data,
        globalState
      );
    });

    it("retrieve-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["defaultRouting"];

      cy.retrieveRoutingConfig(data, globalState);
    });

    it("activate-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["defaultRouting"];

      cy.activateRoutingConfig(data, globalState);
    });

    it("payment-routing-test", () => {
      globalState.set("connectorId", "stripe");
      globalState.set("merchantConnectorId", globalState.get("stripeMcaId"));
      const data =
        utils.getConnectorDetails("stripe")["card_pm"]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest({ globalState });
    });
  });

  context(
    "Rule-based routing - fallback to defaultSelection when rule does not match",
    () => {
      before("seed global state", () => {
        cy.task("getGlobalState").then((state) => {
          globalState = new State(state);
        });
      });

      afterEach("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("list-mca-by-mid", () => {
        cy.ListMcaByMid(globalState);
      });

      it("add-routing-config", () => {
        const data = utils.getConnectorDetails("common")["defaultRouting"];
        // Rule amount > 999999 will never match for a test payment of amount=100
        // defaultSelection routes to adyen as the fallback connector
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
              name: "rule_1",
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
                      lhs: "amount",
                      comparison: "greater_than",
                      value: { type: "number", value: 999999 },
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
        const data = utils.getConnectorDetails("common")["defaultRouting"];

        cy.retrieveRoutingConfig(data, globalState);
      });

      it("activate-routing-call-test", () => {
        const data = utils.getConnectorDetails("common")["defaultRouting"];

        cy.activateRoutingConfig(data, globalState);
      });

      // amount=100 does NOT match rule (amount > 999999), routed to adyen via defaultSelection
      it("payment-routing-test", () => {
        globalState.set("connectorId", "adyen");
        globalState.set("merchantConnectorId", globalState.get("adyenMcaId"));
        const data =
          utils.getConnectorDetails("adyen")["card_pm"]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest({ globalState });
      });
    }
  );
});
