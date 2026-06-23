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
});
