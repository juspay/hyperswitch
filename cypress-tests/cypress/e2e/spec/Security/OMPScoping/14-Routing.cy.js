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
let profileIdPA;
let profileIdPB;
let routingConfigIdPA;

describe("OMP Scoping - Routing", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      routingConfigIdPA = globalState.get("routingConfigIdPA");

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
