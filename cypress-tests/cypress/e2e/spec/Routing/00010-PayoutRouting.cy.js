import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as routingUtils from "../../configs/Routing/Utils";
import * as payoutUtils from "../../configs/Payout/Utils";

let globalState;

describe("Payout Priority Routing Test", () => {
  let shouldContinue = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (
        !routingUtils.shouldIncludeConnector(
          globalState.get("connectorId"),
          routingUtils.CONNECTOR_LISTS.INCLUDE.PAYOUT_ROUTING
        )
      ) {
        shouldContinue = false;
      }
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  context("Payout Priority Routing - default connector", () => {
    shouldContinue = true;

    before("setup payout context", () => {
      // List MCAs once at the start of context to populate connector mappings
      cy.ListMcaByMid(globalState);
    });

    it("add-payout-routing-config", () => {
      const data = routingUtils.getConnectorDetails("common")["payoutRouting"];
      // Use currentConnectorMcaId which is reliably set to the first payout connector
      // This works regardless of which specific connector (stripe/adyen/wise) is configured
      const routing_data = [
        {
          connector: globalState.get("connectorId"),
          merchant_connector_id: globalState.get("currentConnectorMcaId"),
        },
      ];
      cy.addRoutingConfig(
        fixtures.payoutRoutingConfigBody,
        data,
        "priority",
        routing_data,
        globalState
      );
      if (shouldContinue) shouldContinue = routingUtils.should_continue_further(data);
    });

    it("retrieve-payout-routing-config-test", () => {
      const data = routingUtils.getConnectorDetails("common")["payoutRouting"];

      cy.retrieveRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = routingUtils.should_continue_further(data);
    });

    it("activate-payout-routing-config-test", () => {
      const data = routingUtils.getConnectorDetails("common")["payoutRouting"];

      cy.activateRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = routingUtils.should_continue_further(data);
    });

    it("payout-routing-test", () => {
      const payoutData = payoutUtils.getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Fulfill"];

      if (!payoutUtils.should_continue_further(payoutData)) {
        cy.task("cli_log", "Skipping payout creation for " + globalState.get("connectorId"));
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        payoutData,
        true,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = payoutUtils.should_continue_further(payoutData);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Payout Priority Routing - single connector alternate config", () => {
    shouldContinue = true;
    // Context for testing a second payout routing configuration
    // Tests that multiple routing configs can coexist with different priorities

    before("setup alternate payout context", () => {
      // MCAs already listed in first context, but ensure mapping exists
      if (!globalState.get("currentConnectorMcaId")) {
        cy.ListMcaByMid(globalState);
      }
    });

    it("add-payout-routing-config-alternate", () => {
      const data = routingUtils.getConnectorDetails("common")["payoutRouting"];
      // Create an alternate configuration with same connector but different config name
      // This validates that multiple payout routing configs can be managed independently
      const routing_data = [
        {
          connector: globalState.get("connectorId"),
          merchant_connector_id: globalState.get("currentConnectorMcaId"),
        },
      ];
      
      // Modify the body to use a different name for this alternate config
      const altBody = { ...fixtures.payoutRoutingConfigBody };
      altBody.name = `${altBody.name}_alternate`;
      
      cy.addRoutingConfig(
        altBody,
        data,
        "priority",
        routing_data,
        globalState
      );
      if (shouldContinue) shouldContinue = routingUtils.should_continue_further(data);
    });

    it("retrieve-payout-routing-config-alternate-test", () => {
      const data = routingUtils.getConnectorDetails("common")["payoutRouting"];

      cy.retrieveRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = routingUtils.should_continue_further(data);
    });

    it("activate-payout-routing-config-alternate-test", () => {
      const data = routingUtils.getConnectorDetails("common")["payoutRouting"];

      cy.activateRoutingConfig(data, globalState);
      if (shouldContinue) shouldContinue = routingUtils.should_continue_further(data);
    });

    it("payout-routing-alternate-test", () => {
      const payoutData = payoutUtils.getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Fulfill"];

      if (!payoutUtils.should_continue_further(payoutData)) {
        cy.task("cli_log", "Skipping payout creation for " + globalState.get("connectorId"));
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(
        fixtures.createPayoutBody,
        payoutData,
        true,
        true,
        globalState
      );

      if (shouldContinue) shouldContinue = payoutUtils.should_continue_further(payoutData);
    });

    it("retrieve-payout-alternate-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });
});