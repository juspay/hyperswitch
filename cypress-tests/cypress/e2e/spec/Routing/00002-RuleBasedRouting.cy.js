import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

describe("Rule Based Routing Test", () => {
  let shouldContinue = true;

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
    cy.session("login", () => {
      if (!globalState.get("email") || !globalState.get("password")) {
        throw new Error("Missing login credentials in global state");
      }

      cy.userLogin(globalState)
        .then(() => cy.terminate2Fa(globalState))
        .then(() => cy.userInfo(globalState))
        .then(() => {
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

  context(
    "Rule based routing - payment_method: card→Stripe, bank_redirect→Adyen",
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
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-routing-call-test", () => {
        const data = utils.getConnectorDetails("common")["ruleBasedRouting"];
        cy.retrieveRoutingConfig(data, globalState);
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("activate-routing-call-test", () => {
        const data = utils.getConnectorDetails("common")["ruleBasedRouting"];
        cy.activateRoutingConfig(data, globalState);
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
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
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest({ globalState });
      });

      it("create-payment-routing-test for bank redirect", () => {
        const data =
          utils.getConnectorDetails("adyen")["bank_redirect_pm"][
            "PaymentIntent"
          ];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState
        );
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
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
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Handle bank redirect redirection", () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        const payment_method_type = globalState.get("paymentMethodType");
        cy.handleBankRedirectRedirection(
          globalState,
          payment_method_type,
          expected_redirection
        );
      });
    }
  );

  context("Rule based routing - currency: USD→Stripe, else→Adyen", () => {
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
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["ruleBasedRouting"];
      cy.retrieveRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("activate-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["ruleBasedRouting"];
      cy.activateRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
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
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Confirm No 3DS - USD→Stripe", () => {
      const data =
        utils.getConnectorDetails("stripe")["card_pm"]["No3DSAutoCapture"];
      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test-USD", () => {
      cy.retrievePaymentCallTest({ globalState });
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
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Confirm No 3DS - EUR→Adyen", () => {
      const data =
        utils.getConnectorDetails("adyen")["card_pm"]["No3DSAutoCapture"];
      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test-EUR", () => {
      cy.retrievePaymentCallTest({ globalState });
    });
  });

  context("Rule based routing - amount: >1000→Stripe, <1000→Adyen", () => {
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
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["ruleBasedRouting"];
      cy.retrieveRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("activate-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["ruleBasedRouting"];
      cy.activateRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("create-payment-call-test-with-amount-above-1000", () => {
      const data =
        utils.getConnectorDetails("stripe")["card_pm"]["PaymentIntent"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Confirm No 3DS - amount>1000→Stripe", () => {
      const data =
        utils.getConnectorDetails("stripe")["card_pm"]["No3DSAutoCapture"];
      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test-high-amount", () => {
      cy.retrievePaymentCallTest({ globalState });
    });

    it("create-payment-call-test-with-amount-below-1000", () => {
      const data =
        utils.getConnectorDetails("adyen")["card_pm"]["PaymentIntent"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Confirm No 3DS - amount<1000→Adyen", () => {
      const data =
        utils.getConnectorDetails("adyen")["card_pm"]["No3DSAutoCapture"];
      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test-low-amount", () => {
      cy.retrievePaymentCallTest({ globalState });
    });
  });
});
