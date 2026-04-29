import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

describe("Volume Based Routing Test", () => {
  context("Volume based routing with 100% of stripe", () => {
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
      const data = utils.getConnectorDetails("common")["volumeBasedRouting"];
      const routing_data = [
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
        data,
        "volume_split",
        routing_data,
        globalState
      );
    });

    it("retrieve-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["volumeBasedRouting"];

      cy.retrieveRoutingConfig(data, globalState);
    });

    it("activate-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["volumeBasedRouting"];

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

    it("create-payment-call-test-for-eps", () => {
      const data =
        utils.getConnectorDetails("stripe")["bank_redirect_pm"][
          "PaymentIntent"
        ];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
      const data =
        utils.getConnectorDetails("stripe")["bank_redirect_pm"]["eps"];

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
      // confirmBankRedirectCallTest overwrites connectorId via updateConnectorState; restore it
      // to "stripe" here because the 100% stripe routing config guarantees a stripe redirect URL.
      globalState.set("connectorId", "stripe");
      globalState.set("merchantConnectorId", globalState.get("stripeMcaId"));
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

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("list-mca-by-mid", () => {
      cy.ListMcaByMid(globalState);
    });

    it("add-routing-config", () => {
      const data = utils.getConnectorDetails("common")["volumeBasedRouting"];
      const routing_data = [
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
        data,
        "volume_split",
        routing_data,
        globalState
      );
    });

    it("retrieve-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["volumeBasedRouting"];

      cy.retrieveRoutingConfig(data, globalState);
    });

    it("activate-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["volumeBasedRouting"];

      cy.activateRoutingConfig(data, globalState);
    });

    it("payment-routing-test-for-card", () => {
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

    it("create-payment-call-test-for-eps", () => {
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

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm bank redirect", () => {
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

  context("Volume based routing with 50% stripe / 50% adyen", () => {
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
      const data = utils.getConnectorDetails("common")["volumeBasedRouting"];
      const routing_data = [
        {
          connector: {
            connector: "stripe",
            merchant_connector_id: globalState.get("stripeMcaId"),
          },
          split: 50,
        },
        {
          connector: {
            connector: "adyen",
            merchant_connector_id: globalState.get("adyenMcaId"),
          },
          split: 50,
        },
      ];

      cy.addRoutingConfig(
        fixtures.routingConfigBody,
        data,
        "volume_split",
        routing_data,
        globalState
      );
    });

    it("retrieve-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["volumeBasedRouting"];

      cy.retrieveRoutingConfig(data, globalState);
    });

    it("activate-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["volumeBasedRouting"];

      cy.activateRoutingConfig(data, globalState);
    });

    it("payment-routing-test-1", () => {
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

    it("retrieve-payment-call-test-1", () => {
      cy.retrievePaymentCallTest({ globalState });
    });

    it("payment-routing-test-2", () => {
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

    it("retrieve-payment-call-test-2", () => {
      cy.retrievePaymentCallTest({ globalState });
    });
  });
});
