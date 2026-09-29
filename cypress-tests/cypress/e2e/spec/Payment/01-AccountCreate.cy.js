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
    // Scoped per merchant (processor_merchant_id), since the untargeted
    // default is "none"/false for all three keys.
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
