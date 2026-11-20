import State from "../../../../utils/State";

// Sync validates that its connector_id (an MCA) belongs to the request's
// profile_id before passing that profile along to the PayPal MCA update.
// Action URL retrieval and tracking-ID reset stay merchant-only by design
// (not part of the fix), so those two just get a light sanity check here,
// not a security assertion.
//
// All three routes (sync, action_url, reset_tracking_id) return 400 IR_20
// "Connector onboarding flow not supported" for ANY non-PayPal connector,
// confirmed live - the feature is PayPal-only. Crucially, that check fires
// independent of the profile match/mismatch being tested here, so using
// MCA_PA (whatever CYPRESS_CONNECTOR is) would make even the NEGATIVE
// profile-mismatch test "pass" for the wrong reason (always 400, whether
// or not the profile actually matched). MCA_PAYPAL_PA / MCA_PAYPAL_PB
// (00-Setup.cy.js) exist specifically so every test below is attributable
// to the actual profile check, not connector support.
//
// The negative test below targets MCA_PAYPAL_PB (genuinely owned by PB)
// with PA's auth - a real cross-profile attempt. An earlier version of this
// test instead took PA's own MCA_PAYPAL_PA and only varied the request
// body's profile_id field to PB; that returned 200, because the actual
// ownership check compares the MCA's real stored profile against the
// authenticated profile, not an arbitrary profile_id value the caller can
// put in the body - PA's own MCA + PA's own auth is never a real mismatch
// no matter what the body claims.
//
// Routes (verified): POST /connector_onboarding/sync, /action_url,
// /reset_tracking_id - all JWTAuth(MerchantAccountWrite) only, no ApiKeyAuth
// alternative, cfg #[cfg(all(feature = "olap", feature = "v1"))] (default).
let globalState;
let merchantIdM1;
let profileIdPA;
let jwtPA;
let mcaPaypalPaId;
let mcaPaypalPbId;

describe("OMP Scoping - Connector Onboarding", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      merchantIdM1 = globalState.get("merchantIdM1");
      profileIdPA = globalState.get("profileIdPA");
      jwtPA = globalState.get("userInfoTokenPA");
      mcaPaypalPaId = globalState.get("mcaPaypalPaId");
      mcaPaypalPbId = globalState.get("mcaPaypalPbId");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  function fetchMcaPaypalPB() {
    return () =>
      cy.ompRetrieveMca(
        merchantIdM1,
        mcaPaypalPbId,
        { type: "apiKey", apiKey: globalState.get("apiKeyM1") },
        globalState
      );
  }

  it("sync rejects connector/profile mismatch, MCA_PAYPAL_PB unchanged", () => {
    // Not OMP_ERRORS.JWT_PROFILE_MISMATCH (configs/Security/Commons.js):
    // that constant is the generic "JWT's own profile vs. the resource's
    // own stored profile" check (validate_profile_id_from_auth_layer,
    // confirmed on Payments retrieve and Business Profile update). This
    // check is different - it compares the request body's profile_id
    // against the MCA_PAYPAL_PB's real stored profile
    // (utils/connector_onboarding.rs:73), not the JWT's own profile - so
    // its exact error may carry a different message even if it shares the
    // same underlying function. Needs its own live confirmation before
    // asserting an exact expectedError here.
    cy.ompAssertDeniedAndUnchanged({
      fetch: fetchMcaPaypalPB(),
      negativeRequest: () =>
        cy.ompConnectorOnboardingSync(
          { type: "jwt", token: jwtPA, profileId: profileIdPA },
          {
            connector_id: mcaPaypalPbId,
            profile_id: profileIdPA,
            connector: "paypal",
          },
          globalState
        ),
    });
  });

  it("sync with matching profile succeeds (positive)", () => {
    cy.ompConnectorOnboardingSync(
      { type: "jwt", token: jwtPA, profileId: profileIdPA },
      {
        connector_id: mcaPaypalPaId,
        profile_id: profileIdPA,
        connector: "paypal",
      },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
    });
  });

  it("sync on a non-existent connector_id returns 404", () => {
    cy.ompConnectorOnboardingSync(
      { type: "jwt", token: jwtPA, profileId: profileIdPA },
      {
        connector_id: "mca_nonexistent_omp_test",
        profile_id: profileIdPA,
        connector: "paypal",
      },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(404);
    });
  });

  it("malformed profile_id in sync body returns 4xx, not 500", () => {
    cy.ompConnectorOnboardingSync(
      { type: "jwt", token: jwtPA, profileId: profileIdPA },
      {
        connector_id: mcaPaypalPaId,
        profile_id: "!!bad!!",
        connector: "paypal",
      },
      globalState
    ).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("invalid credential on sync is rejected", () => {
    cy.ompConnectorOnboardingSync(
      { type: "invalid" },
      {
        connector_id: mcaPaypalPaId,
        profile_id: profileIdPA,
        connector: "paypal",
      },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(401);
    });
  });

  context(
    "Unchanged by design - informational only, not a security assertion",
    () => {
      it("action URL retrieval remains merchant-only", () => {
        cy.ompConnectorOnboardingActionUrl(
          { type: "jwt", token: jwtPA },
          {
            connector: "paypal",
            connector_id: mcaPaypalPaId,
            return_url: "https://example.com",
          },
          globalState
        ).then((res) => {
          expect(res.status).to.eq(200);
        });
      });

      it("tracking-ID reset remains merchant-only", () => {
        cy.ompConnectorOnboardingResetTrackingId(
          { type: "jwt", token: jwtPA },
          { connector: "paypal", connector_id: mcaPaypalPaId },
          globalState
        ).then((res) => {
          expect(res.status).to.eq(200);
        });
      });
    }
  );
});
