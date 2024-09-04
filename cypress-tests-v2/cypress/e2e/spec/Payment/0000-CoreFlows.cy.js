import State from "../../utils/State";

let globalState;

describe("Core APIs", () => {
  context("Organization APIs", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Organization create call", () => {});
    it("Organization retrieve call", () => {});
    it("Organization update call", () => {});
    it("Organization retrieve call", () => {});
  });

  context("Merchant account APIs", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Merchant account create call", () => {});
    it("Merchant account retrieve call", () => {});
    it("Merchant account update call", () => {});
    it("Merchant account retrieve call", () => {});
  });

  context("Business profile APIs", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Business profile create call", () => {});
    it("Business profile retrieve call", () => {});
    it("Business profile update call", () => {});
    it("Business profile retrieve call", () => {});
  });

  context.skip("MCA", () => {});

  context.skip("API Key", () => {});

  context.skip("Routing", () => {});
});
