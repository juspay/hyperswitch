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

  it("Create remaining business profiles and merchant connector accounts", () => {
    const multipleConnectors = globalState.get("MULTIPLE_CONNECTORS");
    const connectorCount = multipleConnectors?.status
      ? multipleConnectors.count
      : 0;

    if (connectorCount <= 2) {
      cy.task(
        "cli_log",
        "Skipping additional connector account setup; no extra connector credentials configured."
      );
      return;
    }

    for (
      let connectorIndex = 3;
      connectorIndex <= connectorCount;
      connectorIndex++
    ) {
      const connectorValue = `connector_${connectorIndex}`;

      utils.createBusinessProfile(
        fixtures.businessProfile.bpCreate,
        globalState,
        { nextConnector: true, value: connectorValue }
      );

      utils.createMerchantConnectorAccount(
        "payment_processor",
        fixtures.createConnectorBody,
        globalState,
        payment_methods_enabled,
        { nextConnector: true, value: connectorValue }
      );
    }
  });
});
