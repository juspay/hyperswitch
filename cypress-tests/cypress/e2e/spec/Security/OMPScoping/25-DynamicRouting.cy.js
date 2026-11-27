import State from "../../../../utils/State";

// Dynamic-routing strategies (success-rate based, elimination, contract-based,
// volume-split) are a SEPARATE route tree from the generic /routing endpoints
// covered by 14-Routing.cy.js - gated behind the "dynamic_routing" cargo
// feature, registered under
// /account/{account_id}/business_profile/{profile_id}/dynamic_routing/*
// (app.rs:2722,2734), and accepting EITHER a plain api-key
// (HeaderAuth(ApiKeyAuth)) OR JWTAuthProfileFromRoute (routing.rs:1360-1366
// et al) - not the JWTAuthMerchantAndProfileFromRoute used by the rest of
// Business Profile (see 08-BusinessProfile.cy.js). Never exercised by any
// existing spec before this file - this is new coverage, not a regression
// check.
//
// What this file DOES verify: cross-merchant access is denied, because the
// core handler resolves the target profile via
// core_utils::validate_and_get_business_profile(db, processor, profile_id)
// (core/routing.rs:2545) where `processor` is the authenticated merchant -
// a mismatched merchant_id/profile_id pair should 404/400.
//
// What this file does NOT verify (documented gap, not silently assumed
// safe): whether a user whose JWT is scoped to ONLY profile A can reach
// profile B's dynamic-routing config THROUGH THE SAME MERCHANT, since
// JWTAuthProfileFromRoute takes profile_id from the URL path - unlike
// JWTAuthMerchantAndProfileFromRoute, there's no confirmed cross-check of
// the path profile_id against the token's own narrower profile scope. Can't
// be exercised black-box here: it needs a genuinely profile-scoped (not
// org/merchant-admin) JWT, which requires the real email-invite-accept flow
// this environment doesn't support (see 00-Setup.cy.js's merchant_admin
// bootstrap GAP, same blocker as the Role/Analytics role-scope gaps).
// Tracked in the test matrix as a confirmed-gap row until that's resolved.
let globalState;
let apiKeyM1;
let apiKeyM2;
let merchantIdM1;
let profileIdPA;

describe("OMP Scoping - Dynamic Routing strategies", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      apiKeyM2 = globalState.get("apiKeyM2");
      merchantIdM1 = globalState.get("merchantIdM1");
      profileIdPA = globalState.get("profileIdPA");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  // If the "dynamic_routing" cargo feature isn't compiled into the server
  // under test, every route below simply doesn't exist and every call here
  // returns a plain actix "no matching route" 404 - indistinguishable
  // black-box from a legitimate deny. Logged once so a 404 sweep here isn't
  // mistaken for a denial finding.
  before("log feature-gate caveat", () => {
    cy.task(
      "cli_log",
      "25-DynamicRouting: these routes only exist if the server was built " +
        'with the "dynamic_routing" cargo feature - a 404 here can mean ' +
        "either a correct deny OR the route not existing at all."
    );
  });

  context("Cross-merchant: M2 against M1's profile", () => {
    it("M2 cannot create success-based routing on M1's profile", () => {
      cy.ompCreateSuccessBasedRouting(
        merchantIdM1,
        profileIdPA,
        "none",
        { type: "apiKey", apiKey: apiKeyM2 },
        null,
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("M2 cannot create elimination routing on M1's profile", () => {
      cy.ompCreateEliminationRouting(
        merchantIdM1,
        profileIdPA,
        "none",
        { type: "apiKey", apiKey: apiKeyM2 },
        null,
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("M2 cannot toggle contract-based routing on M1's profile", () => {
      cy.ompContractBasedRoutingToggle(
        merchantIdM1,
        profileIdPA,
        "none",
        { type: "apiKey", apiKey: apiKeyM2 },
        null,
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("M2 cannot set volume split on M1's profile", () => {
      cy.ompSetDynamicRoutingVolumeSplit(
        merchantIdM1,
        profileIdPA,
        50,
        { type: "apiKey", apiKey: apiKeyM2 },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("M2 cannot read volume split on M1's profile", () => {
      cy.ompGetDynamicRoutingVolumeSplit(
        merchantIdM1,
        profileIdPA,
        { type: "apiKey", apiKey: apiKeyM2 },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });
  });

  context("Positive: M1 against its own profile", () => {
    it("M1 can read volume split on its own profile", () => {
      cy.ompGetDynamicRoutingVolumeSplit(
        merchantIdM1,
        profileIdPA,
        { type: "apiKey", apiKey: apiKeyM1 },
        globalState
      ).then((res) => {
        // 200 if a volume-split config already exists for PA, 404 if none
        // was ever set - both are legitimate "no cross-merchant leak"
        // outcomes; a 401/403 here would indicate the positive path itself
        // is broken, which is the only thing this assertion guards against.
        expect(res.status).to.not.be.oneOf([401, 403]);
      });
    });
  });

  context("Edge cases", () => {
    it("invalid credential on dynamic-routing create is rejected", () => {
      cy.ompCreateSuccessBasedRouting(
        merchantIdM1,
        profileIdPA,
        "none",
        { type: "invalid" },
        null,
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });
});
