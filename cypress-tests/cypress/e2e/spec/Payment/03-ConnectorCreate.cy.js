import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;
describe("Connector Account Create flow test", () => {
  let isBankDebitConnector = false;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      // Check if this is a stripe connector (for multi-connector bank debit setup)
      isBankDebitConnector = globalState.get("connectorId") === "stripe";
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("Create merchant connector account", () => {
    cy.createConnectorCallTest(
      "payment_processor",
      fixtures.createConnectorBody,
      payment_methods_enabled,
      globalState
    );
  });

  // subsequent profile and mca ids should check for the existence of multiple connectors
  context(
    "Create another business profile and merchant connector account if MULTIPLE_CONNECTORS flag is true",
    () => {
      it("Create business profile", () => {
        utils.createBusinessProfile(
          fixtures.businessProfile.bpCreate,
          globalState,
          { nextConnector: true }
        );
      });

      it("Create merchant connector account", () => {
        utils.createMerchantConnectorAccount(
          "payment_processor",
          fixtures.createConnectorBody,
          globalState,
          payment_methods_enabled,
          { nextConnector: true }
        );
      });
    }
  );

  context(
    "Create business profile and merchant connector account for connector_3",
    () => {
      beforeEach(function () {
        if (!isBankDebitConnector) {
          this.skip();
        }
      });

      it("Create business profile for connector_3", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState,
          "profile2"
        );
      });

      it("Create merchant connector account for connector_3", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState,
          "profile2",
          "merchantConnector2"
        );
      });
    }
  );

  context(
    "Create business profile and merchant connector account for connector_4",
    () => {
      beforeEach(function () {
        if (!isBankDebitConnector) {
          this.skip();
        }
      });

      it("Create business profile for connector_4", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState,
          "profile3"
        );
      });

      it("Create merchant connector account for connector_4", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState,
          "profile3",
          "merchantConnector3"
        );
      });
    }
  );

  context(
    "Create business profile and merchant connector account for connector_5",
    () => {
      beforeEach(function () {
        if (!isBankDebitConnector) {
          this.skip();
        }
      });

      it("Create business profile for connector_5", () => {
        cy.createBusinessProfileTest(
          fixtures.businessProfile.bpCreate,
          globalState,
          "profile4"
        );
      });

      it("Create merchant connector account for connector_5", () => {
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState,
          "profile4",
          "merchantConnector4"
        );
      });
    }
  );
});
