import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;
let shouldContinue = true;

describe("Dynamic Routing Test", () => {
  context("Success-based dynamic routing", () => {
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
      // Success-based dynamic routing uses decision_engine_configs, not connectors array
      cy.addDynamicRoutingConfig(
        fixtures.routingConfigBody,
        data,
        "success_based",
        null,
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
  });

  context("Elimination dynamic routing", () => {
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
      // Elimination dynamic routing uses decision_engine_configs, not connectors array
      cy.addDynamicRoutingConfig(
        fixtures.routingConfigBody,
        data,
        "elimination",
        null,
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
      // Dynamic routing can route to either stripe or adyen
      // The connector field validation is handled in commands.js
      const data =
        utils.getConnectorDetails("adyen")["card_pm"]["No3DSAutoCapture"];
      cy.createConfirmPaymentDynamicRouting(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentDynamicRoutingTest({ globalState });
    });
  });

  context(
    "Success-based toggle endpoint (404 - endpoint not registered)",
    () => {
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
        cy.toggleDynamicRoutingByType("success_based", data, globalState);
      });
    }
  );

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
      cy.toggleDynamicRoutingByType("elimination", data, globalState);
    });
  });
});
