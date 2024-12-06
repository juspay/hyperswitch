import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import { payment_methods_enabled } from "../PaymentUtils/Commons";
import { createProfileAndConnector } from "../PaymentUtils/Utils";

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

  it("connector-create-call-test", () => {
    cy.createConnectorCallTest(
      "payment_processor",
      fixtures.createConnectorBody,
      payment_methods_enabled,
      globalState
    );
  });

  // subsequent profile and mca ids should check for the existence of multiple connectors
  it("check and create multiple connectors", () => {
    createProfileAndConnector(fixtures, globalState, payment_methods_enabled);
  });
});
