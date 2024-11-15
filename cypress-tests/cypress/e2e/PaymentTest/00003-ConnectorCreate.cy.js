import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import { payment_methods_enabled } from "../PaymentUtils/Commons";

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

  it("check and create multiple connectors", () => {
    const multiple_connectors = Cypress.env("MULTIPLE_CONNECTORS");
    if (multiple_connectors.status) {
      // Create multiple connectors based on the count
      // The first connector is already created when creating merchant account, so start from 1
      for (let i = 1; i < multiple_connectors.count; i++) {
        cy.createBusinessProfileTest(
          fixtures.createBusinessProfile,
          globalState,
          "profile" + i
        );
        cy.createConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          payment_methods_enabled,
          globalState,
          "profile" + i,
          "merchantConnector" + i
        );
      }
    }
  });
});
