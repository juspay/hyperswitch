import createConnectorBody from "../../fixtures/create-connector-body.json";
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
      createConnectorBody,
      payment_methods_enabled,
      globalState
    );
  });
});
