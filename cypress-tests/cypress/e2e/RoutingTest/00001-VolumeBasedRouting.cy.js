import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import * as utils from "../RoutingUtils/Utils";

let globalState;

describe("Volume Based Routing Test", () => {
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

  context("Volume based routing with 100% of stripe", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("list-mca-by-mid", () => {
      cy.ListMcaByMid(globalState);
    });

    it("api-key-create-call-test", () => {
      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
    });

    it("customer-create-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("add-routing-config", () => {
      let data = utils.getConnectorDetails("common")["volumeBasedRouting"];
      let req_data = data["Request"];
      let res_data = data["Response"];

      let routing_data = [
        {
          connector: {
            connector: "stripe",
            merchant_connector_id: globalState.get("stripeMcaId"),
          },
          split: 100,
        },
      ];

      cy.addRoutingConfig(
        fixtures.routingConfigBody,
        req_data,
        res_data,
        "volume_split",
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
      let data = utils.getConnectorDetails("common")["volumeBasedRouting"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.activateRoutingConfig(req_data, res_data, globalState);
    });

    it("payment-routing-test", () => {
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
      cy.retrievePaymentCallTest(globalState);
    });

    it("create-payment-call-test-for-eps", () => {
      let data =
        utils.getConnectorDetails("stripe")["bank_redirect_pm"][
          "PaymentIntent"
        ];
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

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      let data = utils.getConnectorDetails("stripe")["bank_redirect_pm"]["eps"];
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

  context("Volume based routing with 100% of adyen", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });
    it("list-mca-by-mid", () => {
      cy.ListMcaByMid(globalState);
    });

    it("api-key-create-call-test", () => {
      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
    });

    it("customer-create-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("add-routing-config", () => {
      let data = utils.getConnectorDetails("common")["volumeBasedRouting"];
      let req_data = data["Request"];
      let res_data = data["Response"];

      let routing_data = [
        {
          connector: {
            connector: "adyen",
            merchant_connector_id: globalState.get("adyenMcaId"),
          },
          split: 100,
        },
      ];

      cy.addRoutingConfig(
        fixtures.routingConfigBody,
        req_data,
        res_data,
        "volume_split",
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
      let data = utils.getConnectorDetails("common")["volumeBasedRouting"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.activateRoutingConfig(req_data, res_data, globalState);
    });

    it("payment-routing-test-for-card", () => {
      let data =
        utils.getConnectorDetails("adyen")["card_pm"]["No3DSAutoCapture"];
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
      cy.retrievePaymentCallTest(globalState);
    });

    it("create-payment-call-test-for-eps", () => {
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

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      let data = utils.getConnectorDetails("adyen")["bank_redirect_pm"]["eps"];
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
});
