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
// Create and update both assert the correct, secure behavior (denial) -
// refund_create_core's authenticated-profile parameter is literally named
// `_profile_id` (crates/router/src/core/refunds.rs) and refund_update_core
// has the same gap, so both of these are expected to fail against the
// current server rather than pass. Leaving them asserting denial (not the
// current buggy behavior) means they fail honestly and visibly until the
// missing profile check is added upstream, instead of a green checkmark
// on incorrect behavior.
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

  context("Update", () => {
    it("PA updates PB's refund - denied", () => {
      cy.ompUpdateRefund(
        refundIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPB },
        { metadata: { attempted_by: "PB" } },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });
  });

  context("Create", () => {
    it("create a refund in PB for a payment that exists in PA - denied", () => {
      cy.ompCreateRefund(
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPB },
        { payment_id: capturedPaymentIdPA, reason: "omp_scoping_poc" },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });
  });
});
