import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;
let shouldContinue = true;

describe("Priority Based Routing Test", () => {
  context("Routing with Stripe as top priority", () => {
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
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["priorityRouting"];

      cy.retrieveRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("activate-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["priorityRouting"];

      cy.activateRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest({ globalState });
    });
  });

  context("Routing with adyen as top priority", () => {
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
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["priorityRouting"];

      cy.retrieveRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("activate-routing-call-test", () => {
      const data = utils.getConnectorDetails("common")["priorityRouting"];

      cy.activateRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
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
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest({ globalState });
    });
  });
});
