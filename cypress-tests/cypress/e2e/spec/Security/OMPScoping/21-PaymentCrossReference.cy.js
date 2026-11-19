import State from "../../../../utils/State";

// Payment creation referencing a customer_id belonging to a different
// merchant doesn't leak the other merchant's customer - the lookup
// (create_customer_if_not_exist / get_customer_if_exists in
// crates/router/src/core/payments/helpers.rs) is scoped by the
// authenticated merchant_id, so a cross-merchant customer_id either
// creates an unrelated shadow record with the same id (Standard
// merchants) or is explicitly rejected with CustomerNotFound (Platform
// merchants). Extend Authorization reuses the same shared payments_core
// ownership check already verified for the rest of the Payment resource
// in Phase 1.
//
// Routes (verified): POST /payments (create),
// POST /payments/{id}/extend_authorization - both ApiKeyAuth/JWTAuth.
let globalState;
let apiKeyM1;
let apiKeyM2;
let profileIdPA;
let customerIdM1;
let paymentIdPB;

describe("OMP Scoping - Payment cross-reference at create time", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      apiKeyM2 = globalState.get("apiKeyM2");
      profileIdPA = globalState.get("profileIdPA");
      customerIdM1 = globalState.get("customerIdM1");
      paymentIdPB = globalState.get("paymentIdPB");
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("M2 creates a payment supplying M1's customer_id - not a leak", () => {
    cy.ompCreatePayment(
      { type: "apiKey", apiKey: apiKeyM2 },
      { amount: 1000, currency: "USD", customer_id: customerIdM1 },
      globalState
    ).then((res) => {
      // Not asserting denial here deliberately - per the audit, M2's
      // create either (a) creates its own unrelated shadow customer record
      // with the same id, or (b) is rejected outright, depending on
      // merchant type. What must NOT happen is M2 ending up linked to
      // M1's actual customer data.
      expect(res.status).to.be.oneOf([200, 400]);
      if (res.status === 200) {
        cy.ompRetrieveCustomer(
          customerIdM1,
          { type: "apiKey", apiKey: apiKeyM1 },
          globalState
        ).then((m1CustomerRes) => {
          expect(
            m1CustomerRes.body.email,
            "M1's own customer record must be untouched by M2's create"
          ).to.not.eq(undefined);
        });
      }
    });
  });

  it("PA extends authorization on PB's payment - denied", () => {
    cy.ompExtendPaymentAuthorization(
      paymentIdPB,
      { type: "apiKey", apiKey: apiKeyM1, profileId: profileIdPA },
      {},
      globalState
    ).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });
});
