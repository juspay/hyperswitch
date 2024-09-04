import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

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

    it("Organization create call", () => {
      cy.organizationCreateCall(
        fixtures.organization_body.org_create,
        globalState
      );
    });
    it("Organization retrieve call", () => {
      cy.organizationRetrieveCall(globalState);
    });
    it("Organization update call", () => {
      cy.organizationUpdateCall(
        fixtures.organization_body.org_update,
        globalState
      );
    });
    it("Organization retrieve call", () => {
      cy.organizationRetrieveCall(globalState);
    });
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

    it("Merchant account create call", () => {
      cy.merchantAccountCreateCall(
        fixtures.merchant_account_body.ma_create,
        globalState
      );
    });
    it("Merchant account retrieve call", () => {
      cy.merchantAccountRetrieveCall(globalState);
    });
    it("Merchant account update call", () => {
      cy.merchantAccountUpdateCall(
        fixtures.merchant_account_body.ma_update,
        globalState
      );
    });
    it("Merchant account retrieve call", () => {
      cy.merchantAccountRetrieveCall(globalState);
    });
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

    it("Business profile create call", () => {
      cy.businessProfileCreateCall(
        fixtures.business_profile_body.bp_create,
        globalState
      );
    });
    it("Business profile retrieve call", () => {
      cy.businessProfileRetrieveCall(globalState);
    });
    it("Business profile update call", () => {
      cy.businessProfileUpdateCall(
        fixtures.business_profile_body.bp_update,
        globalState
      );
    });
    it("Business profile retrieve call", () => {
      cy.businessProfileRetrieveCall(globalState);
    });
  });

  context.skip("MCA", () => {});

  context.skip("API Key", () => {});

  context.skip("Routing", () => {});
});
