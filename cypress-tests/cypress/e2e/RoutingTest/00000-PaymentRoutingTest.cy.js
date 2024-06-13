import apiKeyCreateBody from "../../fixtures/create-api-key-body.json";
import createConfirmPaymentBody from "../../fixtures/create-confirm-body.json";
import customerCreateBody from "../../fixtures/create-customer-body.json";
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
      console.log("seeding globalState -> " + JSON.stringify(globalState));
      cy.task(
        "cli_log",
        "SEEDING GLOBAL STATE -> " + JSON.stringify(globalState),
      );
    });
  });

  afterEach("flush global state", () => {
    console.log("flushing globalState -> " + JSON.stringify(globalState));
    cy.task("setGlobalState", globalState.data);
    cy.task(
      "cli_log",
      " FLUSHING GLOBAL STATE -> " + JSON.stringify(globalState),
    );
  });

  context("Routing with Stripe as top priority", () => {
    it("create-jwt-token", () => {
      let data = utils.getConnectorDetails("common")["jwt"];
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createJWTToken(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-mca", () => {
      cy.ListMCAbyMID(globalState);
    });

    it("api-key-create-call-test", () => {
    cy.apiKeyCreateTest(apiKeyCreateBody, globalState);
    });

    it("customer-create-call-test", () => {
      cy.createCustomerCallTest(customerCreateBody, globalState);
    });

    it("add-routing-config", () => {
      let data = utils.getConnectorDetails("common")["routing"];
      let req_data = data["Request"];
      let res_data = data["Response"];

      let routing_data = [
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
        routingConfigBody,
        req_data,
        res_data,
        "priority",
        routing_data,
        globalState,
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-routing-call-test", () => {
      let data = utils.getConnectorDetails("common")["routing"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.retrieveRoutingConfig(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("activate-routing-call-test", () => {
      let data = utils.getConnectorDetails("common")["routing"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.activateRoutingConfig(req_data, res_data, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("payment-routing-test", () => {
      let data = utils.getConnectorDetails("common")["card_pm"]["Confirm"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createConfirmPaymentTest(
        createConfirmPaymentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState,
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });
  });
});
