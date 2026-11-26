import State from "../../../../utils/State";

// R5 ("references are access too") from the OMP cross-tenant authorization
// review: pointing a request's BODY at another tenant's object is a
// distinct attack surface from reading/writing that object directly via
// its own endpoint. This file tests the body cross-reference fields
// catalogued in Sheet 3 (Payments/Payouts/Refund/Routing) - mapped by a
// source investigation on 2026-11-13, never previously exercised anywhere
// in this suite.
//
// Scope note: a strict "denied for the right reason" check (no 5xx, pinned
// error code, owner-twin parity - R3 from the same review) is intentionally
// NOT applied here per explicit direction ("R3 - Intentional to skip 5xx
// mapping as it need code fix") - these tests use the same loose
// `status is in 400-499` convention as the rest of this suite. A test here
// passing only proves the reference was rejected, not that it was rejected
// for the specifically-correct reason.
//
// Fields intentionally NOT covered (no usable fixture value exists yet,
// not silently assumed safe):
// - Payments' payment_token / merchant_connector_details.creds_identifier
// - Payouts' payout_token
// These reference a vaulted/out-of-band credential this suite never
// seeded a real value for - flagged as a follow-up, not tested here.
let globalState;
let apiKeyM2;
let profileIdM2Default;
let customerIdM1;
let mandateIdPA;
let paymentMethodIdM1;
let mcaPaId;
let paymentIdPA;
let connectorId;

describe("OMP Scoping - Body reference fields (R5)", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM2 = globalState.get("apiKeyM2");
      profileIdM2Default = globalState.get("profileIdM2Default");
      customerIdM1 = globalState.get("customerIdM1");
      mandateIdPA = globalState.get("mandateIdPA");
      paymentMethodIdM1 = globalState.get("paymentMethodIdM1");
      mcaPaId = globalState.get("mcaPaId");
      paymentIdPA = globalState.get("paymentIdPA");
      connectorId = globalState.get("connectorId");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Payments - customer_id referencing another merchant's customer",
    () => {
      // CORRECTED 2026-11-20 (per explicit clarification): customer_id
      // uniqueness is scoped PER MERCHANT, not global - the same ID
      // string legitimately existing under two different merchants is
      // expected application behavior, not a cross-tenant leak. M2 using
      // customerIdM1's string just means M2 is creating/using its OWN
      // customer record under that same ID in M2's own namespace - it
      // does NOT prove M2 reached M1's actual customer data. The original
      // version of this test asserted the opposite (treating a matching
      // ID as proof of a leak), which was wrong.
      //
      // The real constraint to verify is the other direction: the SAME
      // merchant must not be able to create two different customers
      // sharing one customer_id - that's customers.rs's actual uniqueness
      // boundary, and isn't exercised by this cross-merchant scenario at
      // all.
      it("M2 can use the same customer_id string as M1 (expected - customer_id is merchant-scoped, not global)", () => {
        cy.ompCreatePayment(
          { type: "apiKey", apiKey: apiKeyM2 },
          { amount: 1000, currency: "USD", customer_id: customerIdM1 },
          globalState
        ).then((res) => {
          expect(
            res.status,
            "a merchant using a customer_id string that happens to match another merchant's is not a conflict"
          ).to.not.be.oneOf([401, 403]);
        });
      });
    }
  );

  context(
    "Payments - mandate_id referencing another merchant's mandate",
    () => {
      it("M2 cannot create a payment against M1's mandate_id", function () {
        if (!mandateIdPA) this.skip();

        cy.ompCreatePayment(
          { type: "apiKey", apiKey: apiKeyM2 },
          { amount: 1000, currency: "USD", mandate_id: mandateIdPA },
          globalState
        ).then((res) => {
          expect(res.status).to.be.within(400, 499);
        });
      });
    }
  );

  context(
    "Payments - recurring_details.PaymentMethodId referencing another merchant's PM",
    () => {
      it("M2 cannot charge M1's saved payment method via recurring_details", function () {
        if (!paymentMethodIdM1) this.skip();

        cy.ompCreatePayment(
          { type: "apiKey", apiKey: apiKeyM2 },
          {
            amount: 1000,
            currency: "USD",
            confirm: true,
            off_session: true,
            recurring_details: {
              type: "payment_method_id",
              data: paymentMethodIdM1,
            },
          },
          globalState
        ).then((res) => {
          expect(res.status).to.be.within(400, 499);
        });
      });
    }
  );

  context(
    "Payments - routing.data.merchant_connector_id referencing another merchant's MCA",
    () => {
      // CONFIRMED BUG (live, 2026-11-18): M2's request returns 200 instead
      // of a 4xx denial - M2 can force a payment through M1's connector
      // credentials via the routing override. Asserting the correct
      // secure behavior and letting it fail honestly.
      it("M2 cannot force routing through M1's MCA", function () {
        if (!mcaPaId || !connectorId) this.skip();

        cy.ompCreatePayment(
          { type: "apiKey", apiKey: apiKeyM2 },
          {
            amount: 1000,
            currency: "USD",
            routing: {
              type: "single",
              data: { connector: connectorId, merchant_connector_id: mcaPaId },
            },
          },
          globalState
        ).then((res) => {
          expect(res.status).to.be.within(400, 499);
        });
      });
    }
  );

  context(
    "Payouts - customer_id referencing another merchant's customer",
    () => {
      // CORRECTED 2026-11-20 - same correction as the Payments case
      // above: customer_id is merchant-scoped, not global.
      it("M2 can use the same customer_id string as M1 (expected - customer_id is merchant-scoped, not global)", () => {
        cy.ompCreatePayout(
          { type: "apiKey", apiKey: apiKeyM2 },
          { amount: 1000, currency: "USD", customer_id: customerIdM1 },
          globalState
        ).then((res) => {
          expect(
            res.status,
            "a merchant using a customer_id string that happens to match another merchant's is not a conflict"
          ).to.not.be.oneOf([401, 403]);
        });
      });
    }
  );

  context("Refund - payment_id referencing another merchant's payment", () => {
    it("M2 cannot create a refund against M1's payment_id", function () {
      if (!paymentIdPA) this.skip();

      cy.ompCreateRefund(
        { type: "apiKey", apiKey: apiKeyM2 },
        { payment_id: paymentIdPA, amount: 100, reason: "OMP scoping test" },
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });
  });

  context("Routing - config create referencing another merchant's MCA", () => {
    it("M2 cannot create a routing config naming M1's MCA", function () {
      if (!mcaPaId || !connectorId || !profileIdM2Default) this.skip();

      // M2's OWN profile_id, so this isolates the merchant_connector_id
      // reference specifically - not conflated with a separate
      // cross-merchant profile_id mismatch.
      cy.ompCreateRoutingConfig(
        { type: "apiKey", apiKey: apiKeyM2 },
        profileIdM2Default,
        [{ connector: connectorId, merchant_connector_id: mcaPaId }],
        globalState
      ).then((res) => {
        expect(res.status).to.be.within(400, 499);
      });
    });
  });
});
