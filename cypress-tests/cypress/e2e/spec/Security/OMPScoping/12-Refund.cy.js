import State from "../../../../utils/State";

// Refund has its own profile_id column, independent of the payment_intent
// it's created against.
//
// Routes (verified): GET/POST /refunds/{id}, POST /refunds (create) - all
// ApiKeyAuth or JWTAuth. Retrieve calls validate_profile_id_from_auth_layer
// against refund.profile_id directly - clean. Update has NO profile check
// at all (only merchant-level, via the query filter) - not currently
// exploitable since the update route has no profile-scoped auth path, but
// a real asymmetry worth documenting.
//
// CONFIRMED BUG: refund_create_core's authenticated-profile parameter is
// literally named `_profile_id` (crates/router/src/core/refunds.rs) - the
// leading underscore means the compiler itself flags it unused. It's
// received and never checked against the referenced payment's profile. A
// caller authenticated as profile PB can create a refund for a payment
// that belongs to profile PA, same merchant - this is the most severe
// finding in this audit (an actual money-movement action, not just a data
// read). The test below documents the CURRENT (bad) behavior so it flips
// red -> green once fixed, rather than silently assuming either outcome.
let globalState;
let apiKeyM1;
let profileIdPA;
let profileIdPB;
let capturedPaymentIdPA;
let refundIdPA;

describe("OMP Scoping - Refund", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      capturedPaymentIdPA = globalState.get("capturedPaymentIdPA");
      refundIdPA = globalState.get("refundIdPA");

      if (!capturedPaymentIdPA || !refundIdPA) {
        cy.task(
          "cli_log",
          "Skipping this suite: no capturedPaymentIdPA/refundIdPA in " +
            "globalState - the CIT payment + refund creation steps in " +
            "00-Setup.cy.js may have failed or the configured connector " +
            "may not support the required fixture scenarios."
        );
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!capturedPaymentIdPA || !refundIdPA) {
      this.skip();
    }
  });

  context("Retrieve", () => {
    it("PA retrieves PB's refund - denied", () => {
      cy.ompRetrieveRefund(
        refundIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPB },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("PA retrieves its own refund (positive)", () => {
      cy.ompRetrieveRefund(
        refundIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(200);
        expect(res.body.refund_id).to.eq(refundIdPA);
      });
    });

    it("retrieving a non-existent refund_id returns 404", () => {
      cy.ompRetrieveRefund(
        "ref_nonexistent_omp_test",
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(404);
      });
    });

    it("invalid credential on refund retrieve is rejected", () => {
      cy.ompRetrieveRefund(refundIdPA, { type: "invalid" }, globalState).then(
        (res) => {
          expect(res.status).to.eq(401);
        }
      );
    });
  });

  context("Update - no profile check exists (documented asymmetry)", () => {
    it("PA updates PB's refund - no profile check, live-verify actual result", () => {
      cy.ompUpdateRefund(
        refundIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPB },
        { metadata: { attempted_by: "PB" } },
        globalState
      ).then((res) => {
        // No assertion of denial here deliberately - the audit found no
        // profile_id check on this path at all, only the merchant-scoped
        // query filter. Document whatever the server actually returns.
        cy.task(
          "cli_log",
          `Refund update across profiles returned status ${res.status} - ` +
            "no profile check exists in refund_update_core, this result " +
            "documents current behavior rather than asserting denial."
        );
      });
    });
  });

  context("Create - CONFIRMED BUG", () => {
    it("create a refund in PB for a payment that exists in PA - currently SUCCEEDS (bug)", () => {
      cy.ompCreateRefund(
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPB },
        { payment_id: capturedPaymentIdPA, reason: "omp_scoping_poc" },
        globalState
      ).then((res) => {
        // This asserts the CURRENT (incorrect) behavior: refund_create_core
        // never checks the authenticated profile (PB) against the
        // payment's actual profile (PA), so this succeeds today. Once
        // fixed upstream, this test should be updated to assert denial -
        // until then it documents the live bug rather than silently
        // passing or failing on an assumption.
        expect(
          res.status,
          "refund creation currently succeeds across profiles - this is the bug"
        ).to.eq(200);
        if (res.status === 200) {
          cy.task(
            "cli_log",
            `BUG CONFIRMED: refund ${res.body.refund_id} was created under ` +
              "profile PB for a payment that belongs to profile PA."
          );
        }
      });
    });
  });
});
