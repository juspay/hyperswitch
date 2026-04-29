import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;
let shouldContinue = true;

describe("Dynamic Routing Test", () => {
  context("Success-based dynamic routing with stripe as primary", () => {
    before("seed global state", () => {
      shouldContinue = true;
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

    it("add-success-based-dynamic-routing-config", () => {
      const data = utils.getConnectorDetails("common")["dynamicRouting"];
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
      cy.addDynamicRoutingConfig(
        fixtures.routingConfigBody,
        data,
        "success_based",
        routing_data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-routing-call-test", () => {
      if (!shouldContinue) return;
      const data = utils.getConnectorDetails("common")["dynamicRouting"];
      cy.retrieveRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment-routing-test", () => {
      if (!shouldContinue) return;
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

  context("Elimination dynamic routing with adyen as primary", () => {
    before("seed global state", () => {
      shouldContinue = true;
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

    it("deactivate-previous-routing-config", () => {
      const data =
        utils.getConnectorDetails("common")["deactivateDynamicRouting"];
      cy.deactivateDynamicRoutingConfig("success_based", data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("add-elimination-dynamic-routing-config", () => {
      const data = utils.getConnectorDetails("common")["dynamicRouting"];
      const routing_data = [
        {
          connector: "adyen",
          merchant_connector_id: globalState.get("adyenMcaId"),
        },
        {
          connector: "stripe",
          merchant_connector_id: globalState.get("stripeMcaId"),
        },
      ];
      cy.addDynamicRoutingConfig(
        fixtures.routingConfigBody,
        data,
        "elimination",
        routing_data,
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-routing-call-test", () => {
      if (!shouldContinue) return;
      const data = utils.getConnectorDetails("common")["dynamicRouting"];
      cy.retrieveRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment-routing-test", () => {
      if (!shouldContinue) return;
      const body = { ...fixtures.createConfirmPaymentBody };
      body.authentication_type = "no_three_ds";
      body.capture_method = "automatic";
      body.profile_id = globalState.get("profileId");
      body.customer_id = globalState.get("customerId");
      const adyenData =
        utils.getConnectorDetails("adyen")["card_pm"]["No3DSAutoCapture"];
      for (const key in (adyenData.Request || {})) {
        body[key] = adyenData.Request[key];
      }

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/payments`,
        headers: {
          "Content-Type": "application/json",
          "api-key": globalState.get("apiKey"),
        },
        failOnStatusCode: false,
        body: body,
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body.status).to.equal("succeeded");
        const routedConnector = response.body.connector;
        globalState.set("connectorId", routedConnector);
        globalState.set("paymentID", response.body.payment_id);
        globalState.set("clientSecret", response.body.client_secret);
        globalState.set("paymentAmount", body.amount);
        if (routedConnector === "adyen") {
          globalState.set("merchantConnectorId", globalState.get("adyenMcaId"));
        } else {
          globalState.set("merchantConnectorId", globalState.get("stripeMcaId"));
        }
      });
    });

    it("retrieve-payment-call-test", () => {
      const payment_id = globalState.get("paymentID");
      cy.request({
        method: "GET",
        url: `${globalState.get("baseUrl")}/payments/${payment_id}?force_sync=true`,
        headers: {
          "Content-Type": "application/json",
          "api-key": globalState.get("apiKey"),
        },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.equal(200);
        expect(response.body.payment_id).to.equal(payment_id);
        expect(response.body.status).to.equal("succeeded");
        expect(response.body.connector).to.equal(globalState.get("connectorId"));
      });
    });
  });

  context("Contract-based dynamic routing", () => {
    before("seed global state", () => {
      shouldContinue = true;
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

    it("deactivate-previous-routing-config", () => {
      const data =
        utils.getConnectorDetails("common")["deactivateDynamicRouting"];
      cy.deactivateDynamicRoutingConfig("elimination", data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("add-contract-based-dynamic-routing-config", () => {
      const data = utils.getConnectorDetails("common")["toggleRouting"];
      cy.toggleDynamicRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-routing-call-test", () => {
      if (!shouldContinue) return;
      const data = utils.getConnectorDetails("common")["dynamicRouting"];
      cy.retrieveRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("payment-routing-test", () => {
      if (!shouldContinue) return;
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

  context("Success-based toggle endpoint (404 - endpoint not registered)", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    xit("toggle-success-based-dynamic-routing", () => {
      const data = utils.getConnectorDetails("common")["toggleRouting"];
      const merchantId = globalState.get("merchantId");
      const profileId = globalState.get("profileId");

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/account/${merchantId}/business_profile/${profileId}/dynamic_routing/success_based/toggle?enable=dynamic_connector_selection`,
        headers: {
          "api-key": globalState.get("apiKey"),
          "Content-Type": "application/json",
        },
        failOnStatusCode: false,
        body: {},
      }).then((response) => {
        expect(response.status).to.equal(200);
      });
    });
  });

  context("Elimination toggle endpoint (404 - endpoint not registered)", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    xit("toggle-elimination-dynamic-routing", () => {
      const data = utils.getConnectorDetails("common")["toggleRouting"];
      const merchantId = globalState.get("merchantId");
      const profileId = globalState.get("profileId");

      cy.request({
        method: "POST",
        url: `${globalState.get("baseUrl")}/account/${merchantId}/business_profile/${profileId}/dynamic_routing/elimination/toggle?enable=dynamic_connector_selection`,
        headers: {
          "api-key": globalState.get("apiKey"),
          "Content-Type": "application/json",
        },
        failOnStatusCode: false,
        body: {},
      }).then((response) => {
        expect(response.status).to.equal(200);
      });
    });
  });

  context("Contract config PATCH endpoint (500 - server bug)", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    xit("update-contract-based-dynamic-routing-config", () => {
      const routingConfigId = globalState.get("routingConfigId");
      const merchantId = globalState.get("merchantId");
      const profileId = globalState.get("profileId");

      cy.request({
        method: "PATCH",
        url: `${globalState.get("baseUrl")}/account/${merchantId}/business_profile/${profileId}/dynamic_routing/contracts/config/${routingConfigId}`,
        headers: {
          "api-key": globalState.get("apiKey"),
          "Content-Type": "application/json",
        },
        failOnStatusCode: false,
        body: {
          algorithm_for: "payment",
          connectors: [
            {
              connector: "stripe",
              merchant_connector_id: globalState.get("stripeMcaId"),
            },
          ],
        },
      }).then((response) => {
        expect(response.status).to.equal(200);
      });
    });
  });

  context("Deactivate routing without active config (negative)", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("deactivate-routing-from-previous-context", () => {
      const data =
        utils.getConnectorDetails("common")["deactivateDynamicRouting"];
      cy.deactivateDynamicRoutingConfig("contracts", data, globalState);
    });

    it("deactivate-routing-without-active-config", () => {
      const data =
        utils.getConnectorDetails("common")["deactivateRoutingNegative"];
      cy.deactivateRoutingConfig(data, globalState);
    });
  });
});
