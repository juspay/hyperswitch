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

  context("Health Check", () => {
    it("Create Configs", () => {
      cy.healthCheck(globalState);
    });
  });
});
