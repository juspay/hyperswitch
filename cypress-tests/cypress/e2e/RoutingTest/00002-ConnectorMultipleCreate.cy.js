import createConnectorBody from "../../fixtures/create-connector-body.json";
import State from "../../utils/State";

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

  it("adyen-connector-create-call-test", () => {
    cy.createConnectorWithNameCallTest(
      createConnectorBody,
      "adyen",
      "payment_processor",
      globalState
    );
  });

  it("stripe-connector-create-call-test", () => {
    cy.createConnectorWithNameCallTest(
      createConnectorBody,
      "stripe",
      "payment_processor",
      globalState
    );
  });
});
