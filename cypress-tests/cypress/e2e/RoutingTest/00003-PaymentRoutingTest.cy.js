import createConfirmPaymentBody from "../../fixtures/create-confirm-body.json";
import routingConfigBody from "../../fixtures/routing-config-body.json";
import State from "../../utils/State";
import * as utils from "../RoutingUtils/utils";

let globalState;

describe("Routing Test", () => {
  let should_continue = true; // variable that will be used to skip tests if a previous test fails

  beforeEach(function () {
    if (!should_continue) {
      this.skip();
    }
  });

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Routing with Stripe as top priority", () => {
    it("add-routing-config", () => {
      let data = utils.getConnectorDetails("stripe")["routing"];
      let req_data = data["Request"];
      let res_data = data["Response"];

      let adyen_merchant_connector_id = globalState.get("adyen_mc_id");
      let stripe_merchant_connector_id = globalState.get("stripe_mc_id");

      let routing_data = [
        {
          connector: "stripe",
          merchant_connector_id: `${stripe_merchant_connector_id}`,
        },
        {
          connector: "adyen",
          merchant_connector_id: `${adyen_merchant_connector_id}`,
        },
      ];
      cy.addRoutingConfig(
        routingConfigBody,
        req_data,
        res_data,
        "priority",
        routing_data,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-routing-call-test", () => {
      let data = utils.getConnectorDetails("stripe")["routing"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.retrieveRoutingConfig(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("activate-routing-call-test", () => {
      let data = utils.getConnectorDetails("stripe")["routing"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.activateRoutingConfig(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment-routing-test", () => {
      let data = utils.getConnectorDetails("stripe")["card_pm"]["Confirm"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createConfirmPaymentTest(
        createConfirmPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });
  });
});
