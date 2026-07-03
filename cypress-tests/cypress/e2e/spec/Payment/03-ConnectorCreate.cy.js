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
      structuredClone(fixtures.createConnectorBody),
      payment_methods_enabled,
      globalState
    );
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
