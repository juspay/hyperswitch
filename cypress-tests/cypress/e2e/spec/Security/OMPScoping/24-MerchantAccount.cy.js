import { OMP_ERRORS } from "../../../configs/Security/Commons";
import State from "../../../../utils/State";

// Merchant Account has no profile dimension (merchant-wide only) - these
// tests are cross-MERCHANT isolation only, same shape as 23-ApiKeys.cy.js.
//
// Routes (verified): GET /accounts/{id} (retrieve), POST /accounts/{id}
// (update) - both PlatformOrgAdminAuthWithMerchantIdFromRoute (global admin
// key, unrestricted by path id) or JWTAuthMerchantFromRoute. Identical auth
// struct/check to API Keys above: a JWT's own merchant_id vs the path id
// is compared before any permission check
// (services/authentication.rs:4553-4554), mismatch is 401 IR_17. A plain
// merchant API key is not an accepted credential on these routes.
let globalState;
let adminApiKey;
let merchantIdM1;
let merchantIdM2;
let apiKeyM2;
let organizationIdM1;
let jwtPA;

describe("OMP Scoping - Merchant Account", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      adminApiKey = globalState.get("adminApiKey");
      merchantIdM1 = globalState.get("merchantIdM1");
      merchantIdM2 = globalState.get("merchantIdM2");
      apiKeyM2 = globalState.get("apiKeyM2");
      // Dedicated key, not the generic "organizationId" - that one gets
      // clobbered by every merchant-creation step in 00-Setup.cy.js
      // (M2, M2J, platform/connected/standard all run after M1's signup),
      // so by the time this spec runs it holds whichever merchant was
      // created LAST, never M1's. Confirmed bug, 2026-11-20.
      organizationIdM1 = globalState.get("organizationIdM1");
      jwtPA = globalState.get("userInfoTokenPA");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  function fetchMerchantAccount(merchantId) {
    return () =>
      cy.ompRetrieveMerchantAccount(
        merchantId,
        { type: "adminApiKey", apiKey: adminApiKey },
        globalState
      );
  }

  context("Retrieve", () => {
    it("retrieve with mismatched path merchant_id is rejected", function () {
      if (!jwtPA) this.skip();
      cy.ompRetrieveMerchantAccount(
        merchantIdM2,
        { type: "jwt", token: jwtPA },
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

    it("retrieve with the correct merchant_id succeeds (positive)", function () {
      if (!jwtPA) this.skip();
      cy.ompRetrieveMerchantAccount(
        merchantIdM1,
        { type: "jwt", token: jwtPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
        expect(res.body.merchant_id).to.eq(merchantIdM1);
      });
    });

    it("global admin key works for any merchant_id, unrestricted", () => {
      cy.ompRetrieveMerchantAccount(
        merchantIdM2,
        { type: "adminApiKey", apiKey: adminApiKey },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
      });
    });

    it("retrieving a non-existent merchant_id is rejected (401, by design)", () => {
      // Not a 404 - confirmed live and in source. Auth itself
      // (fetch_merchant_key_store_and_account,
      // services/authentication.rs:2053-2075) fetches the merchant/key_store
      // for the path merchant_id and explicitly maps a not-found DB lookup
      // to ApiErrorResponse::Unauthorized, before the core retrieve logic
      // (which would otherwise return a clean 404) ever runs - deliberately
      // avoids distinguishing "wrong credential" from "merchant doesn't
      // exist" to an unauthenticated-feeling caller.
      cy.ompRetrieveMerchantAccount(
        "merchant_nonexistent_omp_test",
        { type: "adminApiKey", apiKey: adminApiKey },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });

    it("invalid credential on retrieve is rejected", () => {
      cy.ompRetrieveMerchantAccount(
        merchantIdM1,
        { type: "invalid" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });

  context("Update", () => {
    // MerchantAccountUpdate (api_models/src/admin.rs:436-497, V1) requires
    // merchant_id in the BODY - it's deserialized independently of the
    // path param, not inherited from it. It's the only required field;
    // omitting it fails deserialization (400 IR_06) before auth or the
    // merchant-mismatch check ever run, confirmed live. Every body below
    // includes it, matching whatever merchant_id is in the path for that
    // test (deliberately mismatched for the cross-merchant negative test,
    // so the body's own merchant_id is internally consistent with intent).
    it("update with mismatched path merchant_id is rejected, M2 unchanged", function () {
      if (!jwtPA) this.skip();
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchMerchantAccount(merchantIdM2),
        negativeRequest: () =>
          cy.ompUpdateMerchantAccount(
            merchantIdM2,
            { type: "jwt", token: jwtPA },
            { merchant_id: merchantIdM2, merchant_name: "Should be rejected" },
            globalState
          ),
        expectedError: OMP_ERRORS.JWT_MERCHANT_MISMATCH,
      });
    });

    it("update with the correct merchant_id succeeds (positive)", function () {
      if (!jwtPA) this.skip();
      cy.ompUpdateMerchantAccount(
        merchantIdM1,
        { type: "jwt", token: jwtPA },
        { merchant_id: merchantIdM1, merchant_name: "OMP Scoping Test M1" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
      });
    });

    it("invalid credential on update is rejected", () => {
      cy.ompUpdateMerchantAccount(
        merchantIdM1,
        { type: "invalid" },
        { merchant_id: merchantIdM1, merchant_name: "Should be rejected" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });

  // parent_merchant_id (admin.rs:69,471) lets a merchant declare itself a
  // sub-merchant of ANY other merchant_id string. core/admin.rs:1516-1523
  // only checks presence/validity of the target when sub_merchants_enabled
  // is true - no ownership/consent check was found that the named
  // parent_merchant_id belongs to the same org or has agreed to the link.
  // This historically gates shared card-locker access in Hyperswitch, so
  // an unconsented link is a potential real cross-merchant data path, not
  // just a labeling inconsistency - this is the highest-priority finding
  // from the 2026-11-18 selector-mapping pass.
  context("parent_merchant_id (sub-merchant linkage)", () => {
    it("M2 can declare M1 as its parent_merchant_id with no consent from M1", () => {
      // The JWT portion only confirms the unrelated path-mismatch rejection
      // (jwtPA is scoped to M1, not M2) - it's a nice-to-have sanity check,
      // not the actual finding under test, so it's skipped gracefully
      // rather than gating the whole test when no dashboard JWT is
      // available. The real test of this finding needs a credential that's
      // actually allowed to update M2, i.e. the admin key below.
      cy.then(() => {
        if (!jwtPA) return;
        cy.ompUpdateMerchantAccount(
          merchantIdM2,
          { type: "jwt", token: jwtPA },
          {
            merchant_id: merchantIdM2,
            sub_merchants_enabled: true,
            parent_merchant_id: merchantIdM1,
          },
          globalState
        ).then((jwtRes) => {
          expect(jwtRes.status).to.eq(OMP_ERRORS.JWT_MERCHANT_MISMATCH.status);
        });
      });

      cy.ompUpdateMerchantAccount(
        merchantIdM2,
        { type: "adminApiKey", apiKey: adminApiKey },
        {
          merchant_id: merchantIdM2,
          sub_merchants_enabled: true,
          parent_merchant_id: merchantIdM1,
        },
        globalState
      ).then((res) => {
        // Documenting current behavior, not yet asserting a fix: if this
        // succeeds, it CONFIRMS the gap (no consent/ownership check). A
        // product decision is needed on whether parent_merchant_id should
        // require M1's agreement before being accepted for M2.
        expect(
          res.status,
          "parent_merchant_id should require the named parent's consent/ownership before being accepted - if this is 200, the gap is confirmed live"
        ).to.be.within(400, 499);
      });
    });
  });

  context("POST /accounts (create) - PlatformOrgAdminAuth only", () => {
    // CONFIRMED BY SOURCE (2026-11-19): v1 POST /accounts is
    // &auth::PlatformOrgAdminAuth{is_admin_auth_allowed:true,...}
    // (admin.rs:242-245). Re-confirmed 2026-11-20: PlatformOrgAdminAuth
    // does NOT accept a dashboard JWT at all - step 1 is admin-key (with
    // org_id fallback), step 2 is a plain API-KEY lookup requiring the key
    // to belong to a PLATFORM-type merchant specifically
    // (authentication.rs:948-997). So the one legitimate non-admin
    // credential for this route is platformApiKey, not an org-admin JWT -
    // the real org_id-injection test uses that instead.
    it("platform merchant's own key cannot create an account inside a DIFFERENT org (M1's)", function () {
      const platformApiKey = globalState.get("platformApiKey");
      if (!platformApiKey) this.skip();

      cy.ompCreateMerchantAccountRaw(
        { type: "apiKey", apiKey: platformApiKey },
        {
          merchant_name: "Should be rejected - wrong org",
          organization_id: organizationIdM1,
        },
        globalState
      ).then((res) => {
        expect(
          res.status,
          "a platform merchant's own key must not be able to create a merchant inside an unrelated org"
        ).to.be.within(400, 499);
      });
    });

    it("invalid credential on merchant account create is rejected", () => {
      // Confirmed live (2026-11-20): this returns 400, not 401 - likely
      // AdminApiAuthWithApiKeyFallback's fallback path or body validation
      // surfacing before/instead of a hard auth rejection. Exact error
      // code not independently confirmed - asserting denial broadly
      // rather than guessing the precise status.
      cy.ompCreateMerchantAccountRaw(
        { type: "invalid" },
        { merchant_name: "Should be rejected" },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("a plain merchant api key cannot create a merchant account at all", () => {
      // Same caveat as above - confirmed live this is 400, not 401.
      cy.ompCreateMerchantAccountRaw(
        { type: "apiKey", apiKey: apiKeyM2 },
        {
          merchant_name: "Should be rejected",
          organization_id: organizationIdM1,
        },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });
  });
});
