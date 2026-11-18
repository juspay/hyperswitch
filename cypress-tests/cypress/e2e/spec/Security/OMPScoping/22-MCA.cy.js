import * as fixtures from "../../../../fixtures/imports";
import State from "../../../../utils/State";
import { payment_methods_enabled } from "../../../configs/Payment/Commons";
import { OMP_ERRORS } from "../../../configs/Security/Commons";

// MCA_PA / MCA_PB (00-Setup.cy.js) - the shared stripe MCAs under PA/PB.
// Never deleted here (other specs in this folder depend on them); a
// dedicated throwaway MCA is created for the delete positive control.
//
// Routes (verified): GET/POST /account/{merchant_id}/connectors/{mca_id}
// (retrieve/update) both call core_utils::validate_profile_id_from_auth_layer
// (core/admin.rs:3002, :3114) - same check, same error as every other
// nullable-profile resource in this suite, regardless of whether the
// mismatched profile came from a JWT or an API key's X-Profile-Id header.
// DELETE /account/{merchant_id}/connectors/{mca_id} is AdminApiAuth or
// JWTAuthMerchantFromRoute(MerchantConnectorWrite) - confirmed
// profile-agnostic by design (core/admin.rs delete_connector never calls
// the profile check); its only ownership enforcement is a combined
// (merchant_id, mca_id) DB lookup, so a mismatched pair 404s rather than
// deleting the wrong row. GET /account/{merchant_id}/connectors (list) is
// merchant-wide; GET /account/{account_id}/profile/connectors filters by
// the authenticated profile when one is present.
let globalState;
let apiKeyM1;
let merchantIdM1;
let merchantIdM2;
let profileIdPA;
let jwtPA;
let mcaPaId;
let mcaPbId;

describe("OMP Scoping - MCA", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      merchantIdM1 = globalState.get("merchantIdM1");
      merchantIdM2 = globalState.get("merchantIdM2");
      profileIdPA = globalState.get("profileIdPA");
      jwtPA = globalState.get("userInfoTokenPA");
      mcaPaId = globalState.get("mcaPaId");
      mcaPbId = globalState.get("mcaPbId");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  function fetchMca(mcaId) {
    return () =>
      cy.ompRetrieveMca(
        merchantIdM1,
        mcaId,
        { type: "apiKey", apiKey: apiKeyM1 },
        globalState
      );
  }

  context("Retrieve", () => {
    it("retrieve cross-profile MCA_PB denied (API key + X-Profile-Id)", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchMca(mcaPbId),
        negativeRequest: () =>
          cy.ompRetrieveMca(
            merchantIdM1,
            mcaPbId,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            globalState
          ),
        expectedError: OMP_ERRORS.PROFILE_ID_AUTH_MISMATCH,
      });
    });

    it("retrieve cross-profile MCA_PB denied (JWT)", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchMca(mcaPbId),
        negativeRequest: () =>
          cy.ompRetrieveMca(
            merchantIdM1,
            mcaPbId,
            { type: "jwt", token: jwtPA, profileId: profileIdPA },
            globalState
          ),
        expectedError: OMP_ERRORS.PROFILE_ID_AUTH_MISMATCH,
      });
    });

    it("retrieve own MCA_PA with matching profile header succeeds (positive)", () => {
      cy.ompRetrieveMca(
        merchantIdM1,
        mcaPaId,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
        expect(res.body.merchant_connector_id).to.eq(mcaPaId);
      });
    });

    it("retrieve with no profile header succeeds regardless of profile (merchant-wide, positive/compat)", () => {
      cy.ompRetrieveMca(
        merchantIdM1,
        mcaPbId,
        { type: "apiKey", apiKey: apiKeyM1 },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
      });
    });

    it("retrieving a non-existent MCA returns 404", () => {
      cy.ompRetrieveMca(
        merchantIdM1,
        "mca_nonexistent_omp_test",
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("invalid credential on retrieve is rejected", () => {
      cy.ompRetrieveMca(
        merchantIdM1,
        mcaPaId,
        { type: "invalid" },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });

  context("Update", () => {
    // MerchantConnectorUpdate (api_models/src/admin.rs:1991-1997) has
    // #[serde(deny_unknown_fields)] and connector_type is its only
    // non-Option field - omitting it fails deserialization (400 IR_06)
    // before auth or the profile check ever run, confirmed live. Every
    // body below includes it for that reason, not because this suite
    // cares about changing it.
    it("update cross-profile MCA_PB denied, unchanged", () => {
      cy.ompAssertDeniedAndUnchanged({
        fetch: fetchMca(mcaPbId),
        negativeRequest: () =>
          cy.ompUpdateMca(
            merchantIdM1,
            mcaPbId,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            { connector_type: "payment_processor", disabled: false },
            globalState
          ),
        expectedError: OMP_ERRORS.PROFILE_ID_AUTH_MISMATCH,
      });
    });

    it("update own MCA_PA with matching profile header succeeds (positive)", () => {
      cy.ompUpdateMca(
        merchantIdM1,
        mcaPaId,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        { connector_type: "payment_processor", disabled: false },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
      });
    });

    it("invalid credential on update is rejected", () => {
      cy.ompUpdateMca(
        merchantIdM1,
        mcaPaId,
        { type: "invalid" },
        { connector_type: "payment_processor", disabled: false },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });

  context(
    "Delete - never touches MCA_PA/MCA_PB, other specs depend on them",
    () => {
      it("delete with mismatched merchant_id/mca_id combo 404s rather than deleting the wrong row", () => {
        // Admin key (merchant-unbound) + M2's path + MCA_PA's real id (which
        // actually belongs to M1) - confirmed via source that delete_connector
        // does a single combined (merchant_id, mca_id) lookup with no
        // separate profile or secondary ownership check, so a mismatched pair
        // simply doesn't resolve to a row, rather than deleting M1's MCA
        // because the admin key has global reach.
        const adminApiKey = globalState.get("adminApiKey");

        cy.ompDeleteMca(
          merchantIdM2,
          mcaPaId,
          { type: "adminApiKey", apiKey: adminApiKey },
          globalState
        ).then((res) => {
          expect(res.status).to.eq(404);
        });

        cy.ompRetrieveMca(
          merchantIdM1,
          mcaPaId,
          { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
          globalState
        ).then((res) => {
          expect(res.status, "MCA_PA must still exist").to.eq(200);
        });
      });

      it("delete via JWT with mismatched merchant_id is rejected before reaching delete_connector", () => {
        cy.ompDeleteMca(
          merchantIdM2,
          mcaPaId,
          { type: "jwt", token: jwtPA },
          globalState
        ).then((res) => {
          expect(res.status).to.eq(OMP_ERRORS.JWT_MERCHANT_MISMATCH.status);
          expect(res.body?.error?.code).to.eq(
            OMP_ERRORS.JWT_MERCHANT_MISMATCH.code
          );
        });
      });

      it("deletes its own throwaway MCA (positive)", () => {
        // Explicit, unique connector_label - omitting it makes the server
        // derive one from the business profile's own profile_name
        // ("{connector}_{profile_name}"), which collides with any MCA
        // already carrying that exact label (confirmed live: a prior run
        // whose delete step failed left this label taken, causing the
        // next run's create to fail with "already exists" - a self-
        // collision across repeated runs, not a one-off). A unique label
        // per run avoids that regardless of whether a previous run's
        // cleanup succeeded.
        const throwawayConnectorBody = {
          ...structuredClone(fixtures.createConnectorBody),
          connector_label: `stripe_omp_mca_throwaway_${Date.now()}`,
        };

        cy.then(() => {
          globalState.set("merchantId", merchantIdM1);
          globalState.set("apiKey", apiKeyM1);
          globalState.set("paId", profileIdPA);
        });

        cy.createConnectorCallTest(
          "payment_processor",
          throwawayConnectorBody,
          payment_methods_enabled,
          globalState,
          "pa",
          "mcaThrowaway"
        );

        cy.then(() => {
          const adminApiKey = globalState.get("adminApiKey");
          const throwawayMcaId = globalState.get("mcaThrowawayId");

          cy.ompDeleteMca(
            merchantIdM1,
            throwawayMcaId,
            { type: "adminApiKey", apiKey: adminApiKey },
            globalState
          ).then((res) => {
            expect(res.status).to.eq(200);
          });

          // CONFIRMED DEFECT (not a test bug, not caching nuance we
          // should accommodate): delete_connector does a genuine SQL
          // DELETE on this row (core/admin.rs delete_connector ->
          // db.delete_merchant_connector_account_by_merchant_id_merchant_connector_id
          // -> diesel generic_delete, not a soft-delete/flag flip). But its
          // cache invalidation redacts a DIFFERENT key
          // ("{merchant_id}_{profile_id}", storage_impl/src/
          // merchant_connector_account.rs:1122-1172) than the one retrieve
          // reads from ("{merchant_id}_{merchant_connector_id}",
          // same file :582-593) - so GET keeps serving a stale cache hit
          // for the just-deleted mca_id even though the row is genuinely
          // gone from the DB. Asserting the correct expected behavior
          // (404) and letting it fail honestly, per this suite's standing
          // rule, rather than loosening this to accommodate a cache bug.
          cy.ompRetrieveMca(
            merchantIdM1,
            throwawayMcaId,
            { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
            globalState
          ).then((res) => {
            expect(res.status, "deleted MCA must no longer exist").to.eq(404);
          });
        });
      });

      it("invalid credential on delete is rejected", () => {
        cy.ompDeleteMca(
          merchantIdM1,
          mcaPaId,
          { type: "invalid" },
          globalState
        ).then((res) => {
          expect(res.status).to.eq(401);
        });
      });
    }
  );

  context("List", () => {
    it("PA lists M1's connectors, sees both MCA_PA and MCA_PB (merchant-wide, positive)", () => {
      cy.ompListMca(
        merchantIdM1,
        { type: "apiKey", apiKey: apiKeyM1 },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
        const ids = (res.body || []).map((mca) => mca.merchant_connector_id);
        expect(ids).to.include(mcaPaId);
        expect(ids).to.include(mcaPbId);
      });
    });

    it("M2's key cannot list M1's connectors via path manipulation", () => {
      const apiKeyM2 = globalState.get("apiKeyM2");

      cy.ompListMca(
        merchantIdM1,
        { type: "apiKey", apiKey: apiKeyM2 },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("profile-scoped list filters to PA only, excludes MCA_PB", () => {
      cy.ompListProfileConnectors(
        merchantIdM1,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
        const ids = (res.body || []).map((mca) => mca.merchant_connector_id);
        expect(ids).to.include(mcaPaId);
        expect(ids).to.not.include(mcaPbId);
      });
    });
  });
});
