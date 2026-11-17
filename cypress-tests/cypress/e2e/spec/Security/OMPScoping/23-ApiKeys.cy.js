import { OMP_ERRORS } from "../../../configs/Security/Commons";
import State from "../../../../utils/State";

// API Keys have no profile dimension (merchant-wide only) - these tests are
// cross-MERCHANT isolation only, contrasting with most of this folder's
// cross-profile focus.
//
// Routes (verified): POST /api_keys/{merchant_id} (create),
// GET /api_keys/{merchant_id}/list, GET/POST/DELETE
// /api_keys/{merchant_id}/{key_id} (retrieve/update/revoke) - all
// PlatformOrgAdminAuthWithMerchantIdFromRoute (global admin key, unrestricted
// by path merchant_id) or JWTAuthMerchantFromRoute(MerchantApiKey{Read,
// Write}). A JWT's own merchant_id is compared against the path
// merchant_id BEFORE the permission check even runs
// (services/authentication.rs:4553-4554) - mismatch is 401 IR_17 "Access
// forbidden, invalid JWT token was used", not a profile-style 412/400. A
// plain merchant API key is not an accepted credential on these routes at
// all (only JWT or the global admin key, or a platform-account merchant's
// key scoped by org - not applicable to M1/M2 here).
let globalState;
let adminApiKey;
let merchantIdM1;
let merchantIdM2;
let jwtPA;

describe("OMP Scoping - API Keys", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      adminApiKey = globalState.get("adminApiKey");
      merchantIdM1 = globalState.get("merchantIdM1");
      merchantIdM2 = globalState.get("merchantIdM2");
      jwtPA = globalState.get("userInfoTokenPA");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  let throwawayKeyId;

  before(
    "create a throwaway API key under M1 to exercise retrieve/update/revoke against",
    () => {
      cy.ompCreateApiKey(
        merchantIdM1,
        { type: "adminApiKey", apiKey: adminApiKey },
        {
          name: "omp-scoping-apikeys-throwaway",
          expiration: "2069-09-23T01:02:03.000Z",
        },
        globalState
      ).then((res) => {
        expect(res.status, "throwaway API key creation must succeed").to.eq(
          200
        );
        throwawayKeyId = res.body.key_id;
      });
    }
  );

  context("Create", () => {
    it("JWT with mismatched path merchant_id is rejected before the handler runs", () => {
      cy.ompCreateApiKey(
        merchantIdM2,
        { type: "jwt", token: jwtPA },
        { name: "should-be-rejected", expiration: "2069-09-23T01:02:03.000Z" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(OMP_ERRORS.JWT_MERCHANT_MISMATCH.status);
        expect(res.body?.error?.code).to.eq(
          OMP_ERRORS.JWT_MERCHANT_MISMATCH.code
        );
        expect(res.body?.error?.message).to.eq(
          OMP_ERRORS.JWT_MERCHANT_MISMATCH.message
        );
      });
    });

    it("JWT scoped to the correct merchant succeeds (positive)", () => {
      cy.ompCreateApiKey(
        merchantIdM1,
        { type: "jwt", token: jwtPA },
        {
          name: "omp-scoping-jwt-create",
          expiration: "2069-09-23T01:02:03.000Z",
        },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
      });
    });

    it("global admin key works for any merchant_id, unrestricted", () => {
      cy.ompCreateApiKey(
        merchantIdM2,
        { type: "adminApiKey", apiKey: adminApiKey },
        {
          name: "omp-scoping-admin-create-m2",
          expiration: "2069-09-23T01:02:03.000Z",
        },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
      });
    });

    it("invalid credential on create is rejected", () => {
      cy.ompCreateApiKey(
        merchantIdM1,
        { type: "invalid" },
        { name: "should-be-rejected", expiration: "2069-09-23T01:02:03.000Z" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });

  context("Retrieve / update / revoke", () => {
    it("retrieve with mismatched path merchant_id is rejected", () => {
      cy.ompRetrieveApiKey(
        merchantIdM2,
        throwawayKeyId,
        { type: "jwt", token: jwtPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(OMP_ERRORS.JWT_MERCHANT_MISMATCH.status);
        expect(res.body?.error?.code).to.eq(
          OMP_ERRORS.JWT_MERCHANT_MISMATCH.code
        );
      });
    });

    it("retrieve with the correct merchant_id succeeds (positive)", () => {
      cy.ompRetrieveApiKey(
        merchantIdM1,
        throwawayKeyId,
        { type: "jwt", token: jwtPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
        expect(res.body.key_id).to.eq(throwawayKeyId);
      });
    });

    it("update with mismatched path merchant_id is rejected, key unchanged", () => {
      cy.ompRetrieveApiKey(
        merchantIdM1,
        throwawayKeyId,
        { type: "adminApiKey", apiKey: adminApiKey },
        globalState
      ).then((before) => {
        cy.ompUpdateApiKey(
          merchantIdM2,
          throwawayKeyId,
          { type: "jwt", token: jwtPA },
          { name: "should-be-rejected" },
          globalState
        ).then((res) => {
          expect(res.status).to.eq(OMP_ERRORS.JWT_MERCHANT_MISMATCH.status);

          cy.ompRetrieveApiKey(
            merchantIdM1,
            throwawayKeyId,
            { type: "adminApiKey", apiKey: adminApiKey },
            globalState
          ).then((after) => {
            expect(after.body.name).to.eq(before.body.name);
          });
        });
      });
    });

    it("update with the correct merchant_id succeeds (positive)", () => {
      cy.ompUpdateApiKey(
        merchantIdM1,
        throwawayKeyId,
        { type: "jwt", token: jwtPA },
        { name: "omp-scoping-apikeys-throwaway-renamed" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
        expect(res.body.name).to.eq("omp-scoping-apikeys-throwaway-renamed");
      });
    });

    it("revoke with mismatched path merchant_id is rejected, key still active", () => {
      cy.ompRevokeApiKeyAs(
        merchantIdM2,
        throwawayKeyId,
        { type: "jwt", token: jwtPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(OMP_ERRORS.JWT_MERCHANT_MISMATCH.status);
      });

      cy.ompRetrieveApiKey(
        merchantIdM1,
        throwawayKeyId,
        { type: "adminApiKey", apiKey: adminApiKey },
        globalState
      ).then((res) => {
        expect(
          res.status,
          "key must still exist after a rejected revoke"
        ).to.eq(200);
      });
    });

    it("revoke with the correct merchant_id succeeds (positive)", () => {
      cy.ompRevokeApiKeyAs(
        merchantIdM1,
        throwawayKeyId,
        { type: "jwt", token: jwtPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
      });

      cy.ompRetrieveApiKey(
        merchantIdM1,
        throwawayKeyId,
        { type: "adminApiKey", apiKey: adminApiKey },
        globalState
      ).then((res) => {
        expect(res.status, "revoked key must no longer be retrievable").to.eq(
          404
        );
      });
    });

    it("invalid credential on retrieve is rejected", () => {
      cy.ompRetrieveApiKey(
        merchantIdM1,
        throwawayKeyId,
        { type: "invalid" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });

  context("List", () => {
    it("list with mismatched path merchant_id is rejected", () => {
      cy.ompListApiKeys(
        merchantIdM2,
        { type: "jwt", token: jwtPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(OMP_ERRORS.JWT_MERCHANT_MISMATCH.status);
      });
    });

    it("list with the correct merchant_id succeeds (positive)", () => {
      cy.ompListApiKeys(
        merchantIdM1,
        { type: "jwt", token: jwtPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
      });
    });
  });
});
