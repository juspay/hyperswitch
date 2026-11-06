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
let jwtPA;

describe("OMP Scoping - Merchant Account", () => {
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

  function fetchMerchantAccount(merchantId) {
    return () =>
      cy.ompRetrieveMerchantAccount(
        merchantId,
        { type: "adminApiKey", apiKey: adminApiKey },
        globalState
      );
  }

  context("Retrieve", () => {
    it("retrieve with mismatched path merchant_id is rejected", () => {
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

    it("retrieve with the correct merchant_id succeeds (positive)", () => {
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
    it("update with mismatched path merchant_id is rejected, M2 unchanged", () => {
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

    it("update with the correct merchant_id succeeds (positive)", () => {
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
});
