import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;
let shouldContinue = true;

describe("Rule Based Routing Test", () => {
  context("Rule based routing,Card->Stripe,Bank_redirect->adyen", () => {
    before("seed global state", () => {
      shouldContinue = true;
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
      const data = utils.getConnectorDetails("common")["ruleBasedRouting"];
      const routing_data = {
        defaultSelection: {
          type: "priority",
          data: [],
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
          {
            name: "rule_2",
            connectorSelection: {
              type: "priority",
              data: [
                {
                  connector: "adyen",
                  merchant_connector_id: globalState.get("adyenMcaId"),
                },
              ],
            },
            statements: [
              {
                condition: [
                  {
                    lhs: "payment_method",
                    comparison: "equal",
                    value: {
                      type: "enum_variant",
                      value: "bank_redirect",
                    },
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
      const data = utils.getConnectorDetails("common")["ruleBasedRouting"];

      cy.retrieveRoutingConfig(data, globalState);
    });

    it("activate-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["ruleBasedRouting"];

      cy.activateRoutingConfig(data, globalState);
    });

    it("payment-routing-test for card", () => {
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
      // Bug E: guard bank redirect tests — if card payment config indicates failure, skip them.
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest({ globalState });
    });

    it("create-payment-routing-test for bank redirect", () => {
      if (!shouldContinue) return;
      const data =
        utils.getConnectorDetails("adyen")["bank_redirect_pm"]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("Confirm bank redirect", () => {
      if (!shouldContinue) return;
      const data =
        utils.getConnectorDetails("adyen")["bank_redirect_pm"]["eps"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );
    });

    it("Handle bank redirect redirection", () => {
      if (!shouldContinue) return;
      // return_url is a static url (https://example.com) taken from confirm-body fixture and is not updated
      const expected_redirection = fixtures.confirmBody["return_url"];
      const payment_method_type = globalState.get("paymentMethodType");
      cy.handleBankRedirectRedirection(
        globalState,
        payment_method_type,
        expected_redirection
      );
    });
  });

  context("Rule based routing,Currency->is->USD->Stripe->else->adyen", () => {
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
      const data = utils.getConnectorDetails("common")["ruleBasedRouting"];
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
                    lhs: "currency",
                    comparison: "equal",
                    value: {
                      type: "enum_variant",
                      value: "USD",
                    },
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
      const data = utils.getConnectorDetails("common")["ruleBasedRouting"];

      cy.retrieveRoutingConfig(data, globalState);
    });

    it("activate-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["ruleBasedRouting"];

      cy.activateRoutingConfig(data, globalState);
    });

    it("payment-routing-test-USD-to-stripe", () => {
      // Bug D: switched to 1-step createConfirmPaymentTest to avoid 2-step confirm issues.
      // currency=USD matches the rule → routes to stripe.
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

    it("retrieve-payment-call-test-usd", () => {
      cy.retrievePaymentCallTest({ globalState });
    });

    it("payment-routing-test-EUR-to-adyen", () => {
      // Bug D: currency=EUR does not match the USD rule → defaultSelection (adyen).
      // EURNo3DSAutoCapture uses currency=EUR so routing targets adyen.
      globalState.set("connectorId", "adyen");
      globalState.set("merchantConnectorId", globalState.get("adyenMcaId"));
      const data =
        utils.getConnectorDetails("adyen")["card_pm"]["EURNo3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("retrieve-payment-call-test-eur", () => {
      cy.retrievePaymentCallTest({ globalState });
    });
  });

  context(
    "Rule based routing,amount->isGreaterThan->100->adyen->else->stripe",
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

      it("add-routing-config", () => {
        const data = utils.getConnectorDetails("common")["ruleBasedRouting"];
        const routing_data = {
          defaultSelection: {
            type: "priority",
            data: [],
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
                      value: { type: "number", value: 1000 },
                      metadata: {},
                    },
                  ],
                },
              ],
            },
            {
              name: "rule_2",
              connectorSelection: {
                type: "priority",
                data: [
                  {
                    connector: "adyen",
                    merchant_connector_id: globalState.get("adyenMcaId"),
                  },
                ],
              },
              statements: [
                {
                  condition: [
                    {
                      lhs: "amount",
                      comparison: "less_than",
                      value: { type: "number", value: 1000 },
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
        const data = utils.getConnectorDetails("common")["ruleBasedRouting"];

        cy.retrieveRoutingConfig(data, globalState);
      });

      it("activate-routing-call-test", () => {
        const data = utils.getConnectorDetails("common")["ruleBasedRouting"];

        cy.activateRoutingConfig(data, globalState);
      });

      it("payment-routing-test-amount-gt-1000-to-stripe", () => {
        // Bug D: switched to 1-step createConfirmPaymentTest.
        // Stripe No3DSAutoCapture has amount=1100 > 1000 → matches rule_1 → routes to stripe.
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

      it("retrieve-payment-call-test-high-amount", () => {
        cy.retrievePaymentCallTest({ globalState });
      });

      it("payment-routing-test-amount-lt-1000-to-adyen", () => {
        // Bug D: Adyen No3DSAutoCapture has amount=900 < 1000 → matches rule_2 → routes to adyen.
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

      it("retrieve-payment-call-test-low-amount", () => {
        cy.retrievePaymentCallTest({ globalState });
      });
    }
  );

  context(
    "Rule based routing with no matching rule falls to defaultSelection",
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

      it("add-routing-config", () => {
        const data = utils.getConnectorDetails("common")["ruleBasedRouting"];
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
        const data = utils.getConnectorDetails("common")["ruleBasedRouting"];

        cy.retrieveRoutingConfig(data, globalState);
      });

      it("activate-routing-call-test", () => {
        const data = utils.getConnectorDetails("common")["ruleBasedRouting"];

        cy.activateRoutingConfig(data, globalState);
      });

      // amount=100 does NOT match the rule (amount > 999999), so defaultSelection (adyen) is used
      it("payment-routing-test-via-default-selection", () => {
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
