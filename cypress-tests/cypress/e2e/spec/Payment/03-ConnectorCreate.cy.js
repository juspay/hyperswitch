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

    // Nothing on the main payments path used to set `ucs_enabled`; only the
    // UCS-specific spec did. Without it `check_ucs_availability` short-circuits
    // to Disabled and a UCS-only connector never reaches UCS from the flow
    // specs at all, whatever `ucs_only_connectors` says.
    if (
      !utils.shouldIncludeConnector(
        globalState.get("connectorId"),
        utils.CONNECTOR_LISTS.INCLUDE.UCS_CONNECTORS
      )
    ) {
      cy.setupConfigs(globalState, "ucs_enabled", "true");
    }
  });

  it("Create multiple business profiles and merchant connector accounts", () => {
    utils.createBusinessProfilesAndMerchantConnectorAccounts(
      "payment_processor",
      fixtures.createConnectorBody,
      fixtures.businessProfile.bpCreate,
      globalState,
      payment_methods_enabled
    );
  });
});
