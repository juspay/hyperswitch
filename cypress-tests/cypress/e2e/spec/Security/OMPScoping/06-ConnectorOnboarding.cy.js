import State from "../../../../utils/State";

// #15115 - "validate connector profile during onboarding sync". Sync
// validates that its connector_id (an MCA) belongs to the request's
// profile_id before passing that profile along to the PayPal MCA update.
// Action URL retrieval and tracking-ID reset stay merchant-only by design
// (not part of the fix), so those two just get a light sanity check here,
// not a security assertion.
//
// Routes (verified): POST /connector_onboarding/sync, /action_url,
// /reset_tracking_id - all JWTAuth(MerchantAccountWrite) only, no ApiKeyAuth
// alternative, cfg #[cfg(all(feature = "olap", feature = "v1"))] (default).
let globalState;
let baseUrl;
let apiKeyM1;
let merchantIdM1;
let profileIdPA;
let profileIdPB;
let jwtPA;
let mcaPaId;
let connectorName;

describe("OMP Scoping - Connector Onboarding (#15115)", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      baseUrl = globalState.get("baseUrl");
      apiKeyM1 = globalState.get("apiKeyM1");
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
      cy.ompRequest({
        method: "GET",
        url: `${baseUrl}/account/${merchantIdM1}/connectors/${mcaPaId}`,
        auth: { type: "apiKey", apiKey: apiKeyM1 },
      });
  }

  it("TC-15115-01: sync rejects connector/profile mismatch, MCA_PA unchanged", () => {
    cy.ompAssertDeniedAndUnchanged({
      fetch: fetchMcaPA(),
      negativeRequest: () =>
        cy.ompRequest({
          method: "POST",
          url: `${baseUrl}/connector_onboarding/sync`,
          auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
          body: {
            connector_id: mcaPaId,
            profile_id: profileIdPB,
            connector: connectorName,
          },
        }),
    });
  });

  it("TC-15115-02 (positive): sync with matching profile isn't profile-rejected", () => {
    cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/connector_onboarding/sync`,
      auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
      body: {
        connector_id: mcaPaId,
        profile_id: profileIdPA,
        connector: connectorName,
      },
    }).then((res) => {
      // Full onboarding-sync success depends on the connector actually
      // being PayPal, which it may not be in this run - only the
      // profile-scope check is this PR's security surface.
      expect(res.status).to.not.eq(403);
    });
  });

  it("TC-15115-E01: sync on a non-existent connector_id returns 404", () => {
    cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/connector_onboarding/sync`,
      auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
      body: {
        connector_id: "mca_nonexistent_omp_test",
        profile_id: profileIdPA,
        connector: connectorName,
      },
    }).then((res) => {
      expect(res.status).to.eq(404);
    });
  });

  it("TC-15115-E03: malformed profile_id in sync body returns 4xx, not 500", () => {
    cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/connector_onboarding/sync`,
      auth: { type: "jwt", token: jwtPA, profileId: profileIdPA },
      body: {
        connector_id: mcaPaId,
        profile_id: "!!bad!!",
        connector: connectorName,
      },
    }).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("TC-15115-E04: invalid credential on sync is rejected", () => {
    cy.ompRequest({
      method: "POST",
      url: `${baseUrl}/connector_onboarding/sync`,
      auth: { type: "invalid" },
      body: {
        connector_id: mcaPaId,
        profile_id: profileIdPA,
        connector: connectorName,
      },
    }).then((res) => {
      expect(res.status).to.eq(401);
    });
  });

  context(
    "Unchanged by design - informational only, not a security assertion",
    () => {
      it("TC-15115-03: action URL retrieval remains merchant-only", () => {
        cy.ompRequest({
          method: "POST",
          url: `${baseUrl}/connector_onboarding/action_url`,
          auth: { type: "jwt", token: jwtPA },
          body: {
            connector: connectorName,
            connector_id: mcaPaId,
            return_url: "https://example.com",
          },
        }).then((res) => {
          expect(res.status).to.not.eq(401);
        });
      });

      it("TC-15115-04: tracking-ID reset remains merchant-only", () => {
        cy.ompRequest({
          method: "POST",
          url: `${baseUrl}/connector_onboarding/reset_tracking_id`,
          auth: { type: "jwt", token: jwtPA },
          body: { connector: connectorName, connector_id: mcaPaId },
        }).then((res) => {
          expect(res.status).to.not.eq(401);
        });
      });
    }
  );
});
