import fixtures from "../../../fixtures/routing.json";
import State from "../../../utils/State";
import * as utils from "../../configs/Routing/Utils";

let globalState;

// Marked as skipped as the List APIs are not implemented yet.
// In addition to this, we do not want to hard code the MCA Ids in the test cases.
describe("Routing core APIs", () => {
  context("Login", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("User login", () => {
      cy.userLogin(globalState);
      cy.terminate2Fa(globalState);
      cy.userInfo(globalState);
    });
  });

  context("Fetch MCA Ids", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("List MCA call", () => {
      cy.mcaListCall(globalState, "routing");
    });
  });

  context("Routing APIs", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Routing algorithm create call", () => {
      const adyen_merchant_connector_id =
        globalState.data.adyenMerchantConnectorId;
      const bluesnap_merchant_connector_id =
        globalState.data.bluesnapMerchantConnectorId;
      const stripe_merchant_connector_id =
        globalState.data.stripeMerchantConnectorId;

      // Fetching the advanced config details
      const advanced_config_details =
        utils.getServiceDetails("advanced_configs");
      // setting the merchant connector ids in the payload
      // defaultSelection data
      advanced_config_details[
        "data"
      ].defaultSelection.data[0].merchant_connector_id =
        adyen_merchant_connector_id;
      // rules data
      // rule 1
      advanced_config_details[
        "data"
      ].rules[0].connectorSelection.data[0].merchant_connector_id =
        stripe_merchant_connector_id;
      advanced_config_details[
        "data"
      ].rules[0].connectorSelection.data[1].merchant_connector_id =
        bluesnap_merchant_connector_id;
      // rule 2
      advanced_config_details[
        "data"
      ].rules[1].connectorSelection.data[0].merchant_connector_id =
        adyen_merchant_connector_id;

      const payload = {
        name: advanced_config_details["name"],
        data: advanced_config_details["data"],
        description: advanced_config_details["description"],
      };
      const type = "advanced";

      cy.routingSetupCall(fixtures.routing_create, type, payload, globalState);
    });
    it("Routing algorithm activate call", () => {
      cy.routingActivateCall(fixtures.routing_activate, globalState);
    });
    it("Routing algorithm activation retrieve call", () => {
      cy.routingActivationRetrieveCall(globalState);
    });
    it("Routing algorithm deactivate call", () => {
      cy.routingDeactivateCall(globalState);
    });
    it("Routing algorithm retrieve call", () => {
      cy.routingRetrieveCall(globalState);
    });
    it("Routing algorithm default fallback update call", () => {
      //fallback_config_details
      const payload = utils.getServiceDetails("fallback_configs");

      cy.routingDefaultFallbackCall(
        fixtures.default_fallback_update,
        payload,
        globalState
      );
    });
    it("Routing algorithm fallback retrieve call", () => {
      cy.routingFallbackRetrieveCall(globalState);
    });
  });
});
