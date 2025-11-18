import State from "../../../utils/State";

let globalState;

// UCS Setup - Runs after connector creation to enable UCS for all subsequent tests
// Each test run creates a new merchant, so configs are automatically isolated
// No cleanup needed - configs won't affect future test runs

describe("UCS Configuration Setup", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("setup-ucs-configs", () => {
    const connectorId = globalState.get("connectorId");
    cy.setupUCSConfigs(globalState, connectorId);
  });
});

