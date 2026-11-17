import State from "../../../../utils/State";

// Surcharge decision config is a merchant-level resource by design - no
// profile dimension exists in the API at all. The DB key is derived from
// the authenticated merchant's own id, so cross-merchant access is
// structurally impossible, not just checked.
//
// Route (verified): GET/PUT/DELETE /routing/decision/surcharge - JWT auth
// preferred (falling back to api-key only if no JWT is present), matching
// the existing createSurchargeDSLConfig/retrieveSurchargeDSLConfig
// commands.
let globalState;
let apiKeyM2;
let jwtPA;

describe("OMP Scoping - Surcharge Config", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM2 = globalState.get("apiKeyM2");
      jwtPA = globalState.get("userInfoTokenPA");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("M2 reads M1's surcharge config - never returns M1's config", () => {
    cy.ompGetSurchargeConfig(
      { type: "apiKey", apiKey: apiKeyM2 },
      globalState
    ).then((res) => {
      // The surcharge config key is derived from the authenticated
      // merchant's own id, so M2 can only ever reach its own (likely
      // nonexistent) config - never M1's "omp_scoping_surcharge_config".
      if (res.status === 200) {
        expect(res.body.name).to.not.eq("omp_scoping_surcharge_config");
      } else {
        expect(res.status).to.be.within(400, 499);
      }
    });
  });

  it("M2 writes to M1's surcharge config - cannot target M1's key at all", () => {
    cy.ompPutSurchargeConfig(
      { type: "apiKey", apiKey: apiKeyM2 },
      {
        name: "attempted_cross_merchant_overwrite",
        merchant_surcharge_configs: {},
        algorithm: {
          defaultSelection: {
            surcharge_details: {
              surcharge: { type: "rate", value: { percentage: 99 } },
            },
          },
          rules: [],
          metadata: {},
        },
      },
      globalState
    ).then((writeRes) => {
      if (writeRes.status === 200) {
        // Even if M2's own write succeeds, it must only ever have written
        // M2's own key - confirm M1's config is untouched.
        cy.ompGetSurchargeConfig(
          { type: "jwt", token: jwtPA },
          globalState
        ).then((res) => {
          expect(res.status).to.eq(200);
          expect(res.body.name).to.eq("omp_scoping_surcharge_config");
        });
      }
    });
  });

  it("M1 reads its own surcharge config (positive)", () => {
    cy.ompGetSurchargeConfig({ type: "jwt", token: jwtPA }, globalState).then(
      (res) => {
        expect(res.status).to.eq(200);
        expect(res.body.name).to.eq("omp_scoping_surcharge_config");
      }
    );
  });

  it("invalid credential on surcharge read is rejected", () => {
    cy.ompGetSurchargeConfig({ type: "invalid" }, globalState).then((res) => {
      expect(res.status).to.eq(401);
    });
  });
});
