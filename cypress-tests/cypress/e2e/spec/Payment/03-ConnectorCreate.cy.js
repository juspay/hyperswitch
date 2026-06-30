import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Connector Account Create flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
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

  // Create connector_2 (standard multi-connector setup using Utils)
  context(
    "Create business profile and merchant connector account for connector_2",
    () => {
      it("Create business profile for connector_2", () => {
        utils.createBusinessProfile(
          fixtures.businessProfile.bpCreate,
          globalState,
          { nextConnector: true }
        );
      });

      it("Create merchant connector account for connector_2", () => {
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

  // Create connector_3, connector_4, connector_5 using the same Utils pattern
  // The shouldProceedWithOperation check in Utils handles skipping for
  // connectors that don't have MULTIPLE_CONNECTORS enabled (non-Stripe)
  context(
    "Create business profile and merchant connector account for connector_3",
    () => {
      it("Create business profile for connector_3", () => {
        utils.createBusinessProfile(
          fixtures.businessProfile.bpCreate,
          globalState,
          { nextConnector: true, value: "connector_3" }
        );
      });

      it("Create merchant connector account for connector_3", () => {
        utils.createMerchantConnectorAccount(
          "payment_processor",
          fixtures.createConnectorBody,
          globalState,
          payment_methods_enabled,
          { nextConnector: true, value: "connector_3" }
        );
      });
    }
  );

  context(
    "Create business profile and merchant connector account for connector_4",
    () => {
      it("Create business profile for connector_4", () => {
        utils.createBusinessProfile(
          fixtures.businessProfile.bpCreate,
          globalState,
          { nextConnector: true, value: "connector_4" }
        );
      });

      it("Create merchant connector account for connector_4", () => {
        utils.createMerchantConnectorAccount(
          "payment_processor",
          fixtures.createConnectorBody,
          globalState,
          payment_methods_enabled,
          { nextConnector: true, value: "connector_4" }
        );
      });
    }
  );

  context(
    "Create business profile and merchant connector account for connector_5",
    () => {
      it("Create business profile for connector_5", () => {
        utils.createBusinessProfile(
          fixtures.businessProfile.bpCreate,
          globalState,
          { nextConnector: true, value: "connector_5" }
        );
      });

      it("Create merchant connector account for connector_5", () => {
        utils.createMerchantConnectorAccount(
          "payment_processor",
          fixtures.createConnectorBody,
          globalState,
          payment_methods_enabled,
          { nextConnector: true, value: "connector_5" }
        );
      });
    }
  );
});
