import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../../e2e/configs/Payment/Commons";

let globalState;

const blocklistContext = () => ({
  processor_merchant_id: globalState.get("merchantId"),
  provider_merchant_id: globalState.get("merchantId"),
});

const profileBlocklistContext = () => ({
  ...blocklistContext(),
  profile_id: globalState.get("profileId"),
});

const blocklistedCardAllowed = {
  Request: connectorDetails.eligibility_api.BlocklistedCardDenied.Request,
  Response: {
    status: 200,
    body: {},
  },
};

describe("Payments Eligibility API with Blocklist", () => {
  let specShouldSkip = false;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (
        !globalState.get("superpositionBaseUrl") ||
        !globalState.get("superpositionSecret") ||
        !globalState.get("superpositionAuthToken")
      ) {
        cy.task(
          "cli_log",
          "Superposition credentials not set — skipping blocklist spec"
        );
        specShouldSkip = true;
        return;
      }
      expect(globalState.get("merchantId"), "merchant ID").to.be.a("string").and
        .not.be.empty;
      expect(globalState.get("profileId"), "profile ID").to.be.a("string").and
        .not.be.empty;
    });
  });

  beforeEach(function () {
    if (specShouldSkip) {
      this.skip();
    }
  });

  after("cleanup superposition config + flush global state", () => {
    if (!specShouldSkip && globalState?.get("merchantId")) {
      if (globalState.get("profileId")) {
        cy.deleteSuperpositionConfig(globalState, profileBlocklistContext());
      }
      cy.setSuperpositionConfig(
        globalState,
        "payments.payment_blocklist_guard",
        false,
        blocklistContext()
      );
    }
    cy.task("setGlobalState", globalState.data);
  });

  context("Setup Phase", () => {
    it("payment intent create call", () => {
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        connectorDetails.eligibility_api.PaymentIntentForBlocklist,
        "no_three_ds",
        "automatic",
        globalState
      );
    });
  });

  context("Blocklist Configuration", () => {
    it("should create blocklist rule for card_bin 424242", () => {
      cy.blocklistCreateRule(
        fixtures.blocklistCreateBody,
        "424242",
        globalState
      );
    });

    it("should enable blocklist functionality using Superposition", () => {
      cy.setSuperpositionConfig(
        globalState,
        "payments.payment_blocklist_guard",
        true,
        blocklistContext()
      );
    });
  });

  context("Eligibility API Tests", () => {
    it("should deny payment for blocklisted card_bin 424242", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBody,
        connectorDetails.eligibility_api.BlocklistedCardDenied,
        globalState
      );
    });

    it("should allow payment for non-blocklisted card", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBody,
        connectorDetails.eligibility_api.NonBlocklistedCardAllowed,
        globalState
      );
    });
  });

  context("Profile and merchant guard overrides", () => {
    it("should disable the guard for a profile and inherit the enabled merchant guard after cleanup", () => {
      cy.setSuperpositionConfig(
        globalState,
        "payments.payment_blocklist_guard",
        false,
        profileBlocklistContext()
      );
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBody,
        blocklistedCardAllowed,
        globalState
      );

      cy.deleteSuperpositionConfig(globalState, profileBlocklistContext());
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBody,
        connectorDetails.eligibility_api.BlocklistedCardDenied,
        globalState
      );
    });

    it("should enable the guard for a profile and inherit the disabled merchant guard after cleanup", () => {
      cy.setSuperpositionConfig(
        globalState,
        "payments.payment_blocklist_guard",
        false,
        blocklistContext()
      );
      cy.setSuperpositionConfig(
        globalState,
        "payments.payment_blocklist_guard",
        true,
        profileBlocklistContext()
      );
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBody,
        connectorDetails.eligibility_api.BlocklistedCardDenied,
        globalState
      );

      cy.deleteSuperpositionConfig(globalState, profileBlocklistContext());
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBody,
        blocklistedCardAllowed,
        globalState
      );
    });
  });

  context("Cleanup", () => {
    it("should delete blocklist rule", () => {
      cy.blocklistDeleteRule("card_bin", "424242", globalState);
    });
  });
});
