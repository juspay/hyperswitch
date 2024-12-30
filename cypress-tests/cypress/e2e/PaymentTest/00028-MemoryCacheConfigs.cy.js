import State from "../../utils/State";

let globalState;

describe("In Memeory Cache Test", () => {
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
      cy.createConfigs(globalState, key, value);
      cy.fetchConfigs(globalState, key, value);
    });

    it("Update Configs", () => {
      cy.updateConfigs(globalState, key, newValue);
      cy.fetchConfigs(globalState, key, newValue);
    });

    it("delete configs", () => {
      cy.deleteConfigs(globalState, key, newValue);
    });
  });
});
