import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

describe("Rule Based Routing Test", () => {
  // Restore the session if it exists
  beforeEach(() => {
    cy.session("login", () => {
      // Make sure we have credentials
      if (!globalState.get("email") || !globalState.get("password")) {
        throw new Error("Missing login credentials in global state");
      }

      cy.userLogin(globalState)
        .then(() => cy.terminate2Fa(globalState))
        .then(() => cy.userInfo(globalState))
        .then(() => {
          // Verify we have all necessary tokens and IDs
          const requiredKeys = [
            "userInfoToken",
            "merchantId",
            "organizationId",
            "profileId",
          ];
          requiredKeys.forEach((key) => {
            if (!globalState.get(key)) {
              throw new Error(`Missing required key after login: ${key}`);
            }
          });
        });
    });
  });

  context("Get merchant info", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("merchant retrieve call", () => {
      cy.merchantRetrieveCall(globalState);
    });
  });

  context("Rule based routing,Card->Stripe,Bank_redirect->adyen", () => {
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

    it("api-key-create-call-test", () => {
      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
    });

    it("customer-create-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
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
      const data = utils.getConnectorDetails("common")["volumeBasedRouting"];

      cy.retrieveRoutingConfig(data, globalState);
    });

    it("activate-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["ruleBasedRouting"];

      cy.activateRoutingConfig(data, globalState);
    });

    it("payment-routing-test for card", () => {
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
      cy.retrievePaymentCallTest(globalState, null);
    });

    it("create-payment-routing-test for bank redirect", () => {
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
      const data =
        utils.getConnectorDetails("adyen")["bank_redirect_pm"]["ideal"];

      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        data,
        true,
        globalState
      );
    });

    it("Handle bank redirect redirection", () => {
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

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("retrieve-mca", () => {
      cy.ListMcaByMid(globalState);
    });

    it("api-key-create-call-test", () => {
      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
    });

    it("customer-create-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
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
      const data = utils.getConnectorDetails("common")["volumeBasedRouting"];

      cy.retrieveRoutingConfig(data, globalState);
    });

    it("activate-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["ruleBasedRouting"];

      cy.activateRoutingConfig(data, globalState);
    });

    it("create-payment-call-test-with-USD", () => {
      const data =
        utils.getConnectorDetails("stripe")["card_pm"]["PaymentIntent"];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("Confirm No 3DS", () => {
      const data =
        utils.getConnectorDetails("stripe")["card_pm"]["No3DSAutoCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState, null);
    });

    it("create-payment-call-test-with-EUR", () => {
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

    it("Confirm No 3DS", () => {
      const data =
        utils.getConnectorDetails("adyen")["card_pm"]["No3DSAutoCapture"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState, null);
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

      after("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("retrieve-mca", () => {
        cy.ListMcaByMid(globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
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
        const data = utils.getConnectorDetails("common")["volumeBasedRouting"];

        cy.retrieveRoutingConfig(data, globalState);
      });

      it("activate-routing-call-test", () => {
        const data = utils.getConnectorDetails("common")["ruleBasedRouting"];

        cy.activateRoutingConfig(data, globalState);
      });

      it("create-payment-call-test-with-amount-10", () => {
        const data =
          utils.getConnectorDetails("stripe")["card_pm"]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("Confirm No 3DS", () => {
        const data =
          utils.getConnectorDetails("stripe")["card_pm"]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState, null);
      });

      it("create-payment-call-test-with-amount-9", () => {
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

      it("Confirm No 3DS", () => {
        const data =
          utils.getConnectorDetails("adyen")["card_pm"]["No3DSAutoCapture"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        it("retrieve-payment-call-test", () => {
          cy.retrievePaymentCallTest(globalState, null);
        });
      });
    }
  );
});
