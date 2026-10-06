import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

describe("3DS Decision Rule Based Routing Test", () => {
  context(
    "3DS Decision Rule - authentication_type is three_ds routes to Stripe, else to Adyen",
    () => {
      before("seed global state", () => {
        cy.task("getGlobalState").then((state) => {
          globalState = new State(state);
        });
      });

      after("flush global state", () => {
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
                      lhs: "authentication_type",
                      comparison: "equal",
                      value: {
                        type: "enum_variant",
                        value: "three_ds",
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

      it("create-payment-call-test-for-three-ds", () => {
        const data =
          utils.getConnectorDetails("stripe")["card_pm"]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
      });

      it(
        "confirm-three-ds-payment",
        { retries: { runMode: 3, openMode: 0 } },
        () => {
          const data =
            utils.getConnectorDetails("stripe")["card_pm"]["3DSAutoCapture"];

          globalState.set("connectorId", "stripe");
          globalState.set(
            "merchantConnectorId",
            globalState.get("stripeMcaId")
          );
          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
          cy.then(() => {
            expect(
              globalState.get("nextActionUrl"),
              "nextActionUrl must be set by confirm-three-ds-payment"
            ).to.be.a("string").and.not.be.empty;
          });
        }
      );

      it("handle-three-ds-redirection", () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      it("retrieve-payment-call-test-for-three-ds", () => {
        cy.retrievePaymentCallTest({ globalState });
      });

      it("create-payment-call-test-for-no-three-ds", () => {
        const data =
          utils.getConnectorDetails("adyen")["card_pm"]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it(
        "confirm-no-three-ds-payment",
        { retries: { runMode: 0, openMode: 0 } },
        () => {
          const data =
            utils.getConnectorDetails("adyen")["card_pm"][
              "EURNo3DSAutoCapture"
            ];

          globalState.set("connectorId", "adyen");
          globalState.set("merchantConnectorId", globalState.get("adyenMcaId"));
          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        }
      );

      it("retrieve-payment-call-test-for-no-three-ds", () => {
        cy.retrievePaymentCallTest({ globalState });
      });
    }
  );

  context(
    "3DS Decision Rule - three_ds auto capture with Stripe, no_three_ds with Adyen",
    () => {
      before("seed global state", () => {
        cy.task("getGlobalState").then((state) => {
          globalState = new State(state);
        });
      });

      after("flush global state", () => {
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
                      lhs: "authentication_type",
                      comparison: "equal",
                      value: {
                        type: "enum_variant",
                        value: "three_ds",
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

      it("create-payment-call-test-three-ds-auto-capture", () => {
        const data =
          utils.getConnectorDetails("stripe")["card_pm"]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
      });

      it("confirm-three-ds-auto-capture", () => {
        const data =
          utils.getConnectorDetails("stripe")["card_pm"]["3DSAutoCapture"];

        globalState.set("connectorId", "stripe");
        globalState.set("merchantConnectorId", globalState.get("stripeMcaId"));
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });

      it("handle-three-ds-redirection-auto-capture", () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      it("retrieve-three-ds-auto-capture-payment", () => {
        cy.retrievePaymentCallTest({ globalState });
      });

      it("create-payment-call-test-no-three-ds-auto-capture", () => {
        const data =
          utils.getConnectorDetails("adyen")["card_pm"]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it(
        "confirm-no-three-ds-auto-capture",
        { retries: { runMode: 0, openMode: 0 } },
        () => {
          const data =
            utils.getConnectorDetails("adyen")["card_pm"][
              "EURNo3DSAutoCapture"
            ];

          globalState.set("connectorId", "adyen");
          globalState.set("merchantConnectorId", globalState.get("adyenMcaId"));
          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        }
      );

      it("retrieve-no-three-ds-auto-capture-payment", () => {
        cy.retrievePaymentCallTest({ globalState });
      });
    }
  );

  context("3DS Decision Rule - three_ds manual capture flow via Stripe", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
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
                    lhs: "authentication_type",
                    comparison: "equal",
                    value: {
                      type: "enum_variant",
                      value: "three_ds",
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

    it("create-payment-call-test-three-ds-manual-capture", () => {
      const data =
        utils.getConnectorDetails("stripe")["card_pm"]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "manual",
        globalState
      );
    });

    it("confirm-three-ds-manual-capture", () => {
      const data =
        utils.getConnectorDetails("stripe")["card_pm"]["3DSManualCapture"];

      globalState.set("connectorId", "stripe");
      globalState.set("merchantConnectorId", globalState.get("stripeMcaId"));
      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("handle-three-ds-redirection-manual-capture", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it(
      "capture-three-ds-payment",
      { retries: { runMode: 3, openMode: 0 } },
      () => {
        const data = utils.getConnectorDetails("stripe")["card_pm"]["Capture"];
        cy.captureCallTest(fixtures.captureBody, data, globalState);
      }
    );

    it("retrieve-three-ds-manual-capture-payment", () => {
      cy.retrievePaymentCallTest({ globalState });
    });
  });

  context(
    "3DS Decision Rule - combined authentication_type and payment_method conditions",
    () => {
      before("seed global state", () => {
        cy.task("getGlobalState").then((state) => {
          globalState = new State(state);
        });
      });

      after("flush global state", () => {
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
                      lhs: "payment_method",
                      comparison: "equal",
                      value: {
                        type: "enum_variant",
                        value: "card",
                      },
                      metadata: {},
                    },
                    {
                      lhs: "authentication_type",
                      comparison: "equal",
                      value: {
                        type: "enum_variant",
                        value: "three_ds",
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

      it("create-payment-call-test-card-three-ds", () => {
        const data =
          utils.getConnectorDetails("stripe")["card_pm"]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
      });

      it("confirm-card-three-ds", () => {
        const data =
          utils.getConnectorDetails("stripe")["card_pm"]["3DSAutoCapture"];

        globalState.set("connectorId", "stripe");
        globalState.set("merchantConnectorId", globalState.get("stripeMcaId"));
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });

      it("handle-three-ds-redirection-combined-rule", () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      it("retrieve-payment-call-test-card-three-ds", () => {
        cy.retrievePaymentCallTest({ globalState });
      });

      it("create-payment-call-test-card-no-three-ds", () => {
        const data =
          utils.getConnectorDetails("adyen")["card_pm"]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it(
        "confirm-card-no-three-ds",
        { retries: { runMode: 0, openMode: 0 } },
        () => {
          const data =
            utils.getConnectorDetails("adyen")["card_pm"][
              "EURNo3DSAutoCapture"
            ];

          globalState.set("connectorId", "adyen");
          globalState.set("merchantConnectorId", globalState.get("adyenMcaId"));
          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        }
      );

      it("retrieve-payment-call-test-card-no-three-ds", () => {
        cy.retrievePaymentCallTest({ globalState });
      });
    }
  );
});
