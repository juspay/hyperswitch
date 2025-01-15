import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

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
    cy.createPayoutConnectorCallTest(
      "payout_processor",
      fixtures.createConnectorBody,
      globalState
    );
  });
});
