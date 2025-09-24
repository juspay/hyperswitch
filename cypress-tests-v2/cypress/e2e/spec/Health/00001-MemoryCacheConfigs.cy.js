import State from "../../../utils/State";

let globalState;

describe("In Memory Cache configs", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Config flows", () => {
    const key = "test-key";
    const value = "test value";
    const newValue = "new test value";

    it("Create Configs", () => {
      cy.setConfigs(globalState, key, value, "CREATE");
      cy.setConfigs(globalState, key, value, "FETCH");
    });

    it("Update Configs", () => {
      cy.setConfigs(globalState, key, newValue, "UPDATE");
      cy.setConfigs(globalState, key, newValue, "FETCH");
    });

    it("delete configs", () => {
      cy.setConfigs(globalState, key, newValue, "DELETE");
    });
  });
});
