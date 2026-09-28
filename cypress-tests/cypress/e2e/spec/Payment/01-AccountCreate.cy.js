import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;
describe("Account Create flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("merchant-create-call-test", () => {
    cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
  });

  it("set-merchant-offer-engine-config", () => {
    cy.setMerchantOfferEngineConfig(globalState);
  });

  it("enable-offer-engine-for-merchant", () => {
    // offer_engine.enabled/credential_source are targeted context overrides
    // keyed on processor_merchant_id, not the untargeted default — each
    // freshly generated Cypress merchant needs its own override, since the
    // default falls back to "none" for any merchant the override doesn't
    // explicitly target.
    // should_perform_eligibility (PR #13766's secondary change) gates
    // whether the payment-method-list response signals the SDK to run
    // eligibility at all (sdk_next_action.next_action: eligibility_check).
    // Without it, offers_enabled being true doesn't matter — PML always
    // just says "confirm" and 54-OfferEngine.cy.js's direct eligibility
    // calls would be the SDK's only path to it, not the documented gating
    // flow.
    cy.createSuperpositionOverrides(
      globalState,
      {
        "offer_engine.enabled": true,
        "offer_engine.credential_source": "merchant",
        "payments.should_perform_eligibility": true,
      },
      { processor_merchant_id: globalState.get("merchantId") }
    );
  });

  it("api-key-create-call-test", () => {
    cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
  });

  it("create-shadow-config-if-shadow-mode-enabled", () => {
    // Shadow and rollout configs are now merged - create unified config with execution_mode: "shadow"
    cy.createRolloutConfig(globalState);
  });
});
