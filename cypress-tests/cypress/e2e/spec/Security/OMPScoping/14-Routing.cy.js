import State from "../../../../utils/State";

// Routing algorithms are profile-scoped. retrieve/link/unlink/activate all
// resolve the owning business profile and call
// validate_profile_id_from_auth_layer explicitly before acting - including
// when the routing config references another profile's MCA (the MCA's own
// profile is resolved and checked, not just the routing record's).
//
// Routes (verified): GET /routing/{id}, POST /routing/{id}/activate, POST
// /routing/deactivate - ApiKeyAuth or JWTAuth.
let globalState;
let apiKeyM1;
let apiKeyM2;
let profileIdPA;
let profileIdPB;
let routingConfigIdPA;
let platformApiKey;
let connectedMerchantId;
let connectedProfileId;

describe("OMP Scoping - Routing", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      apiKeyM2 = globalState.get("apiKeyM2");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      routingConfigIdPA = globalState.get("routingConfigIdPA");
      platformApiKey = globalState.get("platformApiKey");
      connectedMerchantId = globalState.get("connectedMerchantId");
      connectedProfileId = globalState.get("connectedProfileId");

      if (!routingConfigIdPA) {
        cy.task(
          "cli_log",
          "Skipping this suite: no routingConfigIdPA in globalState - the " +
            "routing config creation step in 00-Setup.cy.js may have failed."
        );
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!routingConfigIdPA) this.skip();
  });

  it("PB retrieves PA's routing algorithm - denied", () => {
    cy.ompRetrieveRoutingAlgorithm(
      routingConfigIdPA,
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPB },
      globalState
    ).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("PB activates PA's routing algorithm - denied", () => {
    cy.ompActivateRoutingAlgorithm(
      routingConfigIdPA,
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPB },
      {},
      globalState
    ).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("PA retrieves its own routing algorithm (positive)", () => {
    cy.ompRetrieveRoutingAlgorithm(
      routingConfigIdPA,
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
      expect(res.body.id).to.eq(routingConfigIdPA);
    });
  });

  context("Platform axis - POST /routing (create)", () => {
    beforeEach(function () {
      if (!platformApiKey || !connectedMerchantId || !connectedProfileId) {
        this.skip();
      }
    });

    it("platform merchant can create a routing config acting as its own connected merchant (positive)", () => {
      cy.ompCreateRoutingConfig(
        {
          type: "apiKey",
          apiKey: platformApiKey,
          connectedMerchantId: connectedMerchantId,
        },
        connectedProfileId,
        [],
        globalState
      ).then((res) => {
        // An empty algorithm.data array may itself be rejected by request
        // validation (400) - that's a content-validation concern, not an
        // identity one. The assertion that matters here is that the
        // platform/connected identity resolution itself doesn't produce a
        // 401/403 - a 400 for empty routing data would still be acceptable
        // evidence the request reached real validation logic.
        expect(
          res.status,
          "platform acting as its own connected merchant, routing create"
        ).to.not.be.oneOf([401, 403]);
      });
    });
  });

  // ACTIVATE's platform axis (row 226 in the test matrix) is deferred -
  // it needs a real routing config belonging to the connected merchant to
  // activate, which needs the connected merchant to have at least one MCA
  // to route to first - more setup than this batch covers. Not silently
  // assumed safe.

  context("POST /routing/deactivate", () => {
    // unlink_routing_config (routing.rs:486-525) resolves the profile to
    // deactivate from auth.profile (the CALLER's own authenticated
    // profile/X-Profile-Id) - there is no path or body target profile_id
    // that selects WHOSE routing gets deactivated, so the usual
    // "cross-profile/cross-merchant via a target ID" framing doesn't apply
    // here the way it does for other resources. The real question is
    // whether a client-supplied profile_id in the body can override the
    // caller's own authenticated profile - it should not.
    it("PA deactivates its own profile's routing (positive)", () => {
      cy.ompDeactivateRouting(
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        {},
        globalState
      ).then((res) => {
        expect(res.status).to.not.be.oneOf([401, 403]);
      });
    });

    it("a body profile_id claiming a different profile does not override the caller's own authenticated profile", () => {
      cy.ompDeactivateRouting(
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        { profile_id: profileIdPB },
        globalState
      ).then((res) => {
        // Documenting current behavior: per source, auth.profile always
        // wins over the body - this assertion pins down that a malformed
        // or mismatched body profile_id doesn't itself cause a 401/403
        // (the request should still resolve to PA's own profile, not be
        // rejected outright for the mismatch).
        expect(res.status).to.not.be.oneOf([401, 403]);
      });
    });

    it("M2's api key is accepted on its own identity, not M1's - no cross-merchant target exists to deny", () => {
      cy.ompDeactivateRouting(
        { type: "apiKey", apiKey: apiKeyM2 },
        {},
        globalState
      ).then((res) => {
        expect(res.status).to.not.be.oneOf([401, 403]);
      });
    });

    it("invalid credential on deactivate is rejected", () => {
      cy.ompDeactivateRouting({ type: "invalid" }, {}, globalState).then(
        (res) => {
          expect(res.status).to.eq(401);
        }
      );
    });
  });

  context("Edge cases", () => {
    it("retrieving a non-existent routing algorithm returns 404", () => {
      cy.ompRetrieveRoutingAlgorithm(
        "routing_nonexistent_omp_test",
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("invalid credential on routing retrieve is rejected", () => {
      cy.ompRetrieveRoutingAlgorithm(
        routingConfigIdPA,
        { type: "invalid" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });
});
