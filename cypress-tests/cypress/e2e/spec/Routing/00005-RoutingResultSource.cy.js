import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

describe("Routing Result Source Test", () => {
  context(
    "routing_result_source not set — defaults to Hyperswitch routing (Stripe first)",
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
        const data = utils.getConnectorDetails("common")["priorityRouting"];
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
        const data = utils.getConnectorDetails("common")["priorityRouting"];
        cy.retrieveRoutingConfig(data, globalState);
      });

      it("activate-routing-call-test", () => {
        const data = utils.getConnectorDetails("common")["priorityRouting"];
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
    }
  );

  context(
    "routing_result_source = hyperswitch_routing — explicitly uses Hyperswitch routing engine",
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

      it("set-routing-result-source-to-hyperswitch-routing", () => {
        const profileId = globalState.get("profileId");
        cy.setupConfigs(
          globalState,
          `routing_result_source_${profileId}`,
          "hyperswitch_routing"
        );
      });

      it("add-routing-config", () => {
        const data = utils.getConnectorDetails("common")["priorityRouting"];
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
        cy.addRoutingConfig(
          fixtures.routingConfigBody,
          data,
          "priority",
          routing_data,
          globalState
        );
      });

      it("retrieve-routing-call-test", () => {
        const data = utils.getConnectorDetails("common")["priorityRouting"];
        cy.retrieveRoutingConfig(data, globalState);
      });

      it("activate-routing-call-test", () => {
        const data = utils.getConnectorDetails("common")["priorityRouting"];
        cy.activateRoutingConfig(data, globalState);
      });

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

      it("delete-routing-result-source-config", () => {
        const profileId = globalState.get("profileId");
        cy.setConfigs(
          globalState,
          `routing_result_source_${profileId}`,
          "hyperswitch_routing",
          "DELETE"
        );
      });
    }
  );

  context(
    "routing_result_source = decision_engine — falls back to Hyperswitch routing when DE unavailable",
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

      it("set-routing-result-source-to-decision-engine", () => {
        const profileId = globalState.get("profileId");
        cy.setupConfigs(
          globalState,
          `routing_result_source_${profileId}`,
          "decision_engine"
        );
      });

      it("add-routing-config", () => {
        const data = utils.getConnectorDetails("common")["priorityRouting"];
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
        const data = utils.getConnectorDetails("common")["priorityRouting"];
        cy.retrieveRoutingConfig(data, globalState);
      });

      it("activate-routing-call-test", () => {
        const data = utils.getConnectorDetails("common")["priorityRouting"];
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

      it("delete-routing-result-source-config", () => {
        const profileId = globalState.get("profileId");
        cy.setConfigs(
          globalState,
          `routing_result_source_${profileId}`,
          "decision_engine",
          "DELETE"
        );
      });
    }
  );
});
