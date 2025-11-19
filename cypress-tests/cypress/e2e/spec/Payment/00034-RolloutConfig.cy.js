import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("Rollout Config Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("should create rollout config", () => {
    cy.createRolloutConfig(globalState);
    cy.task("cli_log", "✅ Rollout config created successfully");
  });

  it("should create shadow rollout config", () => {
    cy.createShadowRolloutConfig(globalState);
    cy.task("cli_log", "✅ Shadow rollout config created successfully");
  });
});
