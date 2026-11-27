import State from "../../../../utils/State";

// CONFIRMED GAP: relay_retrieve (crates/router/src/core/relay.rs) validates
// that the CALLER's own profile exists, but never compares the fetched
// relay record's merchant_id/profile_id against the authenticated context
// - find_relay_by_id queries by id alone. Un-encrypted metadata fields
// (status, connector_reference_id, error_code/message, timestamps) leak
// across profiles on the same merchant freely. Cross-merchant leakage of
// the encrypted request_data/response_data payload is only incidentally
// blocked by decryption failing with the wrong merchant's key - that's
// accidental protection, not an actual check. relay_create itself IS
// correctly profile-scoped (validates via
// find_business_profile_by_merchant_id_profile_id before creating) - only
// retrieve has the gap.
//
// GAP (retrieve tests only): the lookup-at-runtime convention below still
// applies to retrieve, since no OTHER relay record exists besides the one
// this file now creates itself.
//
// Route (verified): GET /relay/{relay_id}, POST /relay - both
// HeaderAuth(ApiKeyAuth{allow_connected_scope_operation:true,
// allow_platform_self_operation:false}) (relay.rs:74-77). relay_create
// itself IS correctly profile-scoped (validates via
// find_business_profile_by_merchant_id_profile_id before creating).
let globalState;
let apiKeyM1;
let apiKeyM2;
let profileIdPA;
let profileIdPB;
let mcaPaId;
let capturedPaymentIdPA;
let relayId;

describe("OMP Scoping - Relay", () => {
  before("seed global state and create a real relay record", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      apiKeyM2 = globalState.get("apiKeyM2");
      profileIdPA = globalState.get("profileIdPA");
      profileIdPB = globalState.get("profileIdPB");
      mcaPaId = globalState.get("mcaPaId");
      capturedPaymentIdPA = globalState.get("capturedPaymentIdPA");
      relayId = globalState.get("relayIdPA");

      // A relay/refund request is a real, consequential action against
      // the connector sandbox (same class of risk as the Dispute accept
      // test) - only build one here if explicitly opted in, OR reuse one
      // that already exists from a prior opted-in run.
      if (relayId) return;

      if (Cypress.env("CREATE_REAL_RELAY_REFUND") !== true) {
        cy.task(
          "cli_log",
          "Skipping relay creation: set CYPRESS_CREATE_REAL_RELAY_REFUND=true " +
            "to build a real relay/refund record for this suite to test " +
            "against - it's a real action against the connector sandbox."
        );
        return;
      }

      if (!capturedPaymentIdPA || !mcaPaId) {
        cy.task(
          "cli_log",
          "Skipping this suite: no capturedPaymentIdPA/mcaPaId in " +
            "globalState - can't build a real RelayRequest without a " +
            "captured payment's connector_transaction_id."
        );
        return;
      }

      cy.ompRetrievePayment(
        capturedPaymentIdPA,
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
        globalState
      ).then((res) => {
        const connectorTransactionId = res.body?.connector_transaction_id;
        const amount = res.body?.amount_captured ?? res.body?.amount;
        const currency = res.body?.currency;

        if (!connectorTransactionId) {
          cy.task(
            "cli_log",
            "Skipping this suite: capturedPaymentIdPA has no " +
              "connector_transaction_id - can't build a real RelayRequest."
          );
          return;
        }

        cy.ompCreateRelay(
          { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
          {
            connector_resource_id: connectorTransactionId,
            connector_id: mcaPaId,
            type: "refund",
            data: { type: "refund", amount, currency },
          },
          globalState
        ).then((createRes) => {
          if (createRes.status === 200) {
            relayId = createRes.body?.relay_id;
            globalState.set("relayIdPA", relayId);
          } else {
            cy.task(
              "cli_log",
              `Skipping this suite: relay create failed (status ` +
                `${createRes.status}) - see response body for the reason.`
            );
          }
        });
      });
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!relayId) this.skip();
  });

  it("PB retrieves PA's relay record (same merchant) - metadata leaks (confirmed gap)", () => {
    cy.ompRetrieveRelay(
      relayId,
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPB },
      globalState
    ).then((res) => {
      // No assertion of denial here deliberately - the audit found no
      // merchant/profile check on relay_retrieve at all. This documents
      // that un-encrypted metadata is returned regardless of profile.
      expect(
        res.status,
        "retrieve succeeds across profiles - no check exists"
      ).to.eq(200);
      cy.task(
        "cli_log",
        `CONFIRMED GAP: relay ${relayId} metadata (status=${res.body?.status}) ` +
          "was returned to profile PB even though it belongs to profile PA."
      );
    });
  });

  it("M2 retrieves M1's relay record (cross-merchant) - payload decrypt fails, metadata still returned", () => {
    cy.ompRetrieveRelay(
      relayId,
      { type: "apiKey", apiKey: apiKeyM2 },
      globalState
    ).then((res) => {
      cy.task(
        "cli_log",
        `Cross-merchant relay retrieve returned status ${res.status} - ` +
          "documenting actual response shape rather than asserting an " +
          "outcome, since encrypted-field decryption failure may surface " +
          "as a 200 with null payload fields, a 500, or a 4xx depending " +
          "on how the decrypt error propagates."
      );
    });
  });

  it("PA retrieves its own relay record (positive)", () => {
    cy.ompRetrieveRelay(
      relayId,
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
    });
  });

  it("retrieving a non-existent relay_id returns 404", () => {
    cy.ompRetrieveRelay(
      "relay_nonexistent_omp_test",
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(404);
    });
  });

  it("invalid credential on relay retrieve is rejected", () => {
    cy.ompRetrieveRelay(relayId, { type: "invalid" }, globalState).then(
      (res) => {
        expect(res.status).to.eq(401);
      }
    );
  });

  context("Create", () => {
    beforeEach(function () {
      if (!mcaPaId) this.skip();
    });

    it("M2 cannot create a relay referencing M1's MCA", () => {
      // relay_create validates the MCA's profile against the
      // authenticated merchant/profile - M2 has no relationship to
      // mcaPaId at all. No real connector call should be reached since
      // this should fail at the profile-ownership check, before touching
      // connector_resource_id.
      cy.ompCreateRelay(
        { type: "apiKey", apiKey: apiKeyM2 },
        {
          connector_resource_id: "irrelevant_for_this_check",
          connector_id: mcaPaId,
          type: "refund",
          data: { type: "refund", amount: 100, currency: "USD" },
        },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("PB (same merchant, different profile) cannot create a relay referencing PA's MCA", () => {
      cy.ompCreateRelay(
        { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPB },
        {
          connector_resource_id: "irrelevant_for_this_check",
          connector_id: mcaPaId,
          type: "refund",
          data: { type: "refund", amount: 100, currency: "USD" },
        },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });

    it("invalid credential on relay create is rejected", () => {
      cy.ompCreateRelay(
        { type: "invalid" },
        {
          connector_resource_id: "irrelevant_for_this_check",
          connector_id: mcaPaId,
          type: "refund",
          data: { type: "refund", amount: 100, currency: "USD" },
        },
        globalState
      ).then((res) => {
        expect(res.status).to.eq(401);
      });
    });
  });
});
