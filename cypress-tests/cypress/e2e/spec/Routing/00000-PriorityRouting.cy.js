import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

describe("Priority Based Routing Test", () => {
  let shouldContinue = true;

  beforeEach(() => {
    cy.session("login", () => {
      // Make sure we have credentials
      if (!globalState.get("email") || !globalState.get("password")) {
        throw new Error("Missing login credentials in global state");
      }

      cy.userLogin(globalState)
        .then(() => cy.terminate2Fa(globalState))
        .then(() => cy.userInfo(globalState))
        .then(() => {
          // Verify we have all necessary tokens and IDs
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

  context("Routing with Stripe as top priority", () => {
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
      cy.retrievePaymentCallTest(globalState, null);
    });
  });

  context("Routing with adyen as top priority", () => {
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
      cy.retrievePaymentCallTest(globalState, null);
    });
  });
});
