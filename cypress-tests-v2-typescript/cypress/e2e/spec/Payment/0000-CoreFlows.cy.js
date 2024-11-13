import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { payment_methods_enabled } from "../../configs/Payment/Commons";

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
    it("Second merchant account create call", () => {
      cy.merchantAccountCreateCall(
        fixtures.merchant_account_body.ma_create,
        globalState
      );
    });
    it("List merchant accounts call", () => {
      cy.merchantAccountsListCall(globalState);
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
    it("Second business profile create call", () => {
      fixtures.business_profile_body.bp_create.profile_name = "HyperSx2";
      cy.businessProfileCreateCall(
        fixtures.business_profile_body.bp_create,
        globalState
      );
    });
    it("List business profiles", () => {
      cy.businessProfilesListCall(globalState);
    });
  });

  context("Merchant connector account APIs", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("[Payment] Merchant connector account create call", () => {
      // `globalState` can only be accessed in the `it` block
      const connector_name = globalState.data.connectorId;
      cy.mcaCreateCall(
        `${connector_name}_default`,
        connector_name,
        "payment_processor",
        globalState,
        fixtures.merchant_connector_account_body.mca_create,
        payment_methods_enabled
      );
    });
    it("[Payment] Merchant connector account retrieve call", () => {
      cy.mcaRetrieveCall(globalState);
    });
    it("[Payment] Merchant connector account update call", () => {
      // `globalState` can only be accessed in the `it` block
      const connector_name = globalState.data.connectorId;
      cy.mcaUpdateCall(
        `${connector_name}_default`,
        connector_name,
        "payment_processor",
        globalState,
        fixtures.merchant_connector_account_body.mca_update,
        payment_methods_enabled
      );
    });
    it("[Payment] Merchant connector accounts list call", () => {
      cy.mcaListCall(globalState, null);
    });
  });

  context("API Key APIs", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("API Key create call", () => {
      cy.apiKeyCreateCall(fixtures.api_key_body.api_key_create, globalState);
    });
    it("API Key retrieve call", () => {
      cy.apiKeyRetrieveCall(globalState);
    });
    it("API Key update call", () => {
      cy.apiKeyUpdateCall(fixtures.api_key_body.api_key_update, globalState);
    });
    it("Second API Key create call", () => {
      cy.apiKeyCreateCall(fixtures.api_key_body.api_key_create, globalState);
    });
    it("API Keys list call", () => {
      cy.apiKeysListCall(globalState);
    });
  });
});
