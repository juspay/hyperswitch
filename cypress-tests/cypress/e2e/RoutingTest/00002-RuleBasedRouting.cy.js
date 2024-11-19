import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import * as utils from "../RoutingUtils/Utils";

let globalState;

describe("Rule Based Routing Test", () => {
  context("Login", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("User login", () => {
      cy.userLogin(globalState);
      cy.terminate2Fa(globalState);
      cy.userInfo(globalState);
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
      let data = utils.getConnectorDetails("common")["ruleBasedRouting"];
      let req_data = data["Request"];
      let res_data = data["Response"];

      let routing_data = {
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
        req_data,
        res_data,
        "advanced",
        routing_data,
        globalState
      );
    });

    it("retrieve-routing-call-test", () => {
      let data = utils.getConnectorDetails("common")["volumeBasedRouting"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.retrieveRoutingConfig(req_data, res_data, globalState);
    });

    it("activate-routing-call-test", () => {
      let data = utils.getConnectorDetails("common")["ruleBasedRouting"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.activateRoutingConfig(req_data, res_data, globalState);
    });

    it("payment-routing-test for card", () => {
      let data =
        utils.getConnectorDetails("stripe")["card_pm"]["No3DSAutoCapture"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState, null);
    });

    it("create-payment-routing-test for bank redirect", () => {
      let data =
        utils.getConnectorDetails("adyen")["bank_redirect_pm"]["PaymentIntent"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("Confirm bank redirect", () => {
      let data =
        utils.getConnectorDetails("adyen")["bank_redirect_pm"]["ideal"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.confirmBankRedirectCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
    });

    it("Handle bank redirect redirection", () => {
      // return_url is a static url (https://hyperswitch.io) taken from confirm-body fixture and is not updated
      let expected_redirection = fixtures.confirmBody["return_url"];
      let payment_method_type = globalState.get("paymentMethodType");
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
      let data = utils.getConnectorDetails("common")["ruleBasedRouting"];
      let req_data = data["Request"];
      let res_data = data["Response"];

      let routing_data = {
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
        req_data,
        res_data,
        "advanced",
        routing_data,
        globalState
      );
    });

    it("retrieve-routing-call-test", () => {
      let data = utils.getConnectorDetails("common")["volumeBasedRouting"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.retrieveRoutingConfig(req_data, res_data, globalState);
    });

    it("activate-routing-call-test", () => {
      let data = utils.getConnectorDetails("common")["ruleBasedRouting"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.activateRoutingConfig(req_data, res_data, globalState);
    });

    it("create-payment-call-test-with-USD", () => {
      let data =
        utils.getConnectorDetails("stripe")["card_pm"]["PaymentIntent"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("Confirm No 3DS", () => {
      let data =
        utils.getConnectorDetails("stripe")["card_pm"]["No3DSAutoCapture"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState, null);
    });

    it("create-payment-call-test-with-EUR", () => {
      let data = utils.getConnectorDetails("adyen")["card_pm"]["PaymentIntent"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("Confirm No 3DS", () => {
      let data =
        utils.getConnectorDetails("adyen")["card_pm"]["No3DSAutoCapture"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
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
        let data = utils.getConnectorDetails("common")["ruleBasedRouting"];
        let req_data = data["Request"];
        let res_data = data["Response"];

        let routing_data = {
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
          req_data,
          res_data,
          "advanced",
          routing_data,
          globalState
        );
      });

      it("retrieve-routing-call-test", () => {
        let data = utils.getConnectorDetails("common")["volumeBasedRouting"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.retrieveRoutingConfig(req_data, res_data, globalState);
      });

      it("activate-routing-call-test", () => {
        let data = utils.getConnectorDetails("common")["ruleBasedRouting"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.activateRoutingConfig(req_data, res_data, globalState);
      });

      it("create-payment-call-test-with-amount-10", () => {
        let data =
          utils.getConnectorDetails("stripe")["card_pm"]["PaymentIntent"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("Confirm No 3DS", () => {
        let data =
          utils.getConnectorDetails("stripe")["card_pm"]["No3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          req_data,
          res_data,
          true,
          globalState
        );
      });

      it("retrieve-payment-call-test", () => {
        cy.retrievePaymentCallTest(globalState, null);
      });

      it("create-payment-call-test-with-amount-9", () => {
        let data =
          utils.getConnectorDetails("adyen")["card_pm"]["PaymentIntent"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("Confirm No 3DS", () => {
        let data =
          utils.getConnectorDetails("adyen")["card_pm"]["No3DSAutoCapture"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          req_data,
          res_data,
          true,
          globalState
        );

        it("retrieve-payment-call-test", () => {
          cy.retrievePaymentCallTest(globalState, null);
        });
      });
    }
  );
});
