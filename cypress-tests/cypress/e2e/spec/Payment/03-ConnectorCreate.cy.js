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

  // Dynamically create connector_2 through connector_5 based on MULTIPLE_CONNECTORS config
  // This avoids hardcoding contexts and allows easy scaling
  [2, 3, 4, 5].forEach((connectorNum) => {
    context(
      `Create business profile and merchant connector account for connector_${connectorNum}`,
      () => {
        it(`Create business profile for connector_${connectorNum}`, () => {
          utils.createBusinessProfile(
            fixtures.businessProfile.bpCreate,
            globalState,
            { nextConnector: true }
          );
        });

        it(`Create merchant connector account for connector_${connectorNum}`, () => {
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
  });
});
