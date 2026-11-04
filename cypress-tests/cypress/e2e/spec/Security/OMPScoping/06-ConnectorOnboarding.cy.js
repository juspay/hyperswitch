import State from "../../../../utils/State";

// Sync validates that its connector_id (an MCA) belongs to the request's
// profile_id before passing that profile along to the PayPal MCA update.
// Action URL retrieval and tracking-ID reset stay merchant-only by design
// (not part of the fix), so those two just get a light sanity check here,
// not a security assertion.
//
// Routes (verified): POST /connector_onboarding/sync, /action_url,
// /reset_tracking_id - all JWTAuth(MerchantAccountWrite) only, no ApiKeyAuth
// alternative, cfg #[cfg(all(feature = "olap", feature = "v1"))] (default).
let globalState;
let merchantIdM1;
let profileIdPA;
let profileIdPB;
let jwtPA;
let mcaPaId;
let connectorName;

describe("OMP Scoping - Connector Onboarding", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      merchantIdM1 = globalState.get("merchantIdM1");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      jwtPA = globalState.get("userInfoTokenPA");
      mcaPaId = globalState.get("mcaPaId");
      // the connector_name MCA_PA was actually created with - set by
      // createConnectorCallTest in 00-Setup.cy.js
      connectorName = globalState.get("originalConnectorId");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  function fetchMcaPA() {
    return () =>
      cy.ompRetrieveMca(
        merchantIdM1,
        mcaPaId,
        { type: "apiKey", apiKey: globalState.get("apiKeyM1") },
        globalState
      );
  }

  it("sync rejects connector/profile mismatch, MCA_PA unchanged", () => {
    cy.ompAssertDeniedAndUnchanged({
      fetch: fetchMcaPA(),
      negativeRequest: () =>
        cy.ompConnectorOnboardingSync(
          { type: "jwt", token: jwtPA, profileId: profileIdPA },
          {
            connector_id: mcaPaId,
            profile_id: profileIdPB,
            connector: connectorName,
          },
          globalState
        ),
    });
  });

  it("sync with matching profile isn't profile-rejected (positive)", () => {
    cy.ompConnectorOnboardingSync(
      { type: "jwt", token: jwtPA, profileId: profileIdPA },
      {
        connector_id: mcaPaId,
        profile_id: profileIdPA,
        connector: connectorName,
      },
      globalState
    ).then((res) => {
      // Full onboarding-sync success depends on the connector actually
      // being PayPal, which it may not be in this run - only the
      // profile-scope check is this security surface.
      expect(res.status).to.not.eq(403);
    });
  });

  it("sync on a non-existent connector_id returns 404", () => {
    cy.ompConnectorOnboardingSync(
      { type: "jwt", token: jwtPA, profileId: profileIdPA },
      {
        connector_id: "mca_nonexistent_omp_test",
        profile_id: profileIdPA,
        connector: connectorName,
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
        connector_id: mcaPaId,
        profile_id: "!!bad!!",
        connector: connectorName,
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
        connector_id: mcaPaId,
        profile_id: profileIdPA,
        connector: connectorName,
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
            connector: connectorName,
            connector_id: mcaPaId,
            return_url: "https://example.com",
          },
          globalState
        ).then((res) => {
          expect(res.status).to.not.eq(401);
        });
      });

      it("tracking-ID reset remains merchant-only", () => {
        cy.ompConnectorOnboardingResetTrackingId(
          { type: "jwt", token: jwtPA },
          { connector: connectorName, connector_id: mcaPaId },
          globalState
        ).then((res) => {
          expect(res.status).to.not.eq(401);
        });
      });
    }
  );
});
