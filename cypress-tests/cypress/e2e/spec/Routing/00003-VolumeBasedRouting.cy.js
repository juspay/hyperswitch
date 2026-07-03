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
      // Bug C fix: dynamically detect which connector the router chose
      // instead of assuming stripe. 50/50 split is probabilistic.
      const baseUrl = globalState.get("baseUrl");
      const apiKey = globalState.get("apiKey");
      const customerId = globalState.get("customerId");
      const profileId = globalState.get("profileId");

      cy.request({
        method: "POST",
        url: `${baseUrl}/payments`,
        headers: {
          "Content-Type": "application/json",
          "api-key": apiKey,
        },
        failOnStatusCode: false,
        body: {
          amount: 6000,
          currency: "USD",
          confirm: true,
          authentication_type: "no_three_ds",
          capture_method: "automatic",
          profile_id: profileId,
          customer_id: customerId,
          payment_method: "card",
          payment_method_type: "debit",
          payment_method_data: {
            card: {
              card_number: "4242424242424242",
              card_exp_month: "01",
              card_exp_year: "50",
              card_holder_name: "Test User",
              card_cvc: "123",
            },
          },
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              line3: "Harrison Street",
              city: "San Fransisco",
              state: "CA",
              zip: "94122",
              country: "US",
            },
          },
          email: "guest@example.com",
          return_url: "https://example.com",
          browser_info: {
            ip_address: "129.0.0.1",
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
            accept_header: "text/html",
            language: "en-US",
          },
        },
      }).then((response) => {
        expect(response.status).to.eq(200);
        const actualConnector = response.body.connector;
        expect(actualConnector).to.be.oneOf(["stripe", "adyen"]);

        globalState.set("connectorId", actualConnector);
        if (actualConnector === "stripe") {
          globalState.set("merchantConnectorId", globalState.get("stripeMcaId"));
        } else {
          globalState.set("merchantConnectorId", globalState.get("adyenMcaId"));
        }
        globalState.set("paymentID", response.body.payment_id);
        globalState.set("paymentAmount", response.body.amount);
      });
    });

    it("retrieve-payment-call-test-1", () => {
      cy.retrievePaymentCallTest({ globalState });
    });

    it("payment-routing-test-2", () => {
      // For 50/50 routing, the connector is probabilistic.
      // We capture the actual routed connector from the response, then use it for subsequent assertions.
      const baseUrl = globalState.get("baseUrl");
      const apiKey = globalState.get("apiKey");
      const customerId = globalState.get("customerId");
      const profileId = globalState.get("profileId");

      cy.request({
        method: "POST",
        url: `${baseUrl}/payments`,
        headers: {
          "Content-Type": "application/json",
          "api-key": apiKey,
        },
        failOnStatusCode: false,
        body: {
          amount: 6000,
          currency: "USD",
          confirm: true,
          authentication_type: "no_three_ds",
          capture_method: "automatic",
          profile_id: profileId,
          customer_id: customerId,
          payment_method: "card",
          payment_method_type: "debit",
          payment_method_data: {
            card: {
              card_number: "4242424242424242",
              card_exp_month: "01",
              card_exp_year: "50",
              card_holder_name: "Test User",
              card_cvc: "123",
            },
          },
          billing: {
            address: {
              line1: "1467",
              line2: "Harrison Street",
              line3: "Harrison Street",
              city: "San Fransisco",
              state: "CA",
              zip: "94122",
              country: "US",
            },
          },
          email: "guest@example.com",
          return_url: "https://example.com",
          browser_info: {
            ip_address: "129.0.0.1",
            user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
            accept_header: "text/html",
            language: "en-US",
          },
        },
      }).then((response) => {
        expect(response.status).to.eq(200);
        const actualConnector = response.body.connector;
        // Accept either stripe or adyen for 50/50 routing
        expect(actualConnector).to.be.oneOf(["stripe", "adyen"]);
        
        // Set the connectorId to match what the server actually routed to
        globalState.set("connectorId", actualConnector);
        // Set the merchant connector id based on which connector was used
        if (actualConnector === "stripe") {
          globalState.set("merchantConnectorId", globalState.get("stripeMcaId"));
        } else {
          globalState.set("merchantConnectorId", globalState.get("adyenMcaId"));
        }
        globalState.set("paymentID", response.body.payment_id);
        globalState.set("paymentAmount", response.body.amount);
      });
    });

    it("retrieve-payment-call-test-2", () => {
      cy.retrievePaymentCallTest({ globalState });
    });
  });
});
