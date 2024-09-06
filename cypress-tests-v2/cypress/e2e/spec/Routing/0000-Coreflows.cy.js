import State from "../../../utils/State";

let globalState;

// Marked as skipped as the List APIs are not implemented yet.
// In addition to this, we do not want to hard code the MCA Ids in the test cases.
describe.skip("Routingh core APIs", () => {
  context("Login", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("User login", () => {
      cy.userLogin(globalState);
      cy.terminate2Fa(globalState);
      cy.userInfo(globalState);
    });
  });

  context("Fetch MCA Ids", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("List MCA call", () => {
      cy.listMCA(globalState);
    });
  });

  context("Routing APIs", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Routing algorithm create call", () => {});
    it("Routing algorithm activate call", () => {});
    it("Routing algorithm retrieve call", () => {});
    it("Routing algorithm deactivate call", () => {});
    it("Routing algorithm retrieve call", () => {});
    it("Routing algorithm default fallback update call", () => {});
    it("Routing algorithm retrieve call", () => {});
  });
});
