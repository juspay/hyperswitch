import State from "../../../../utils/State";

// Payment Link's API-key-authenticated retrieve (payment_link_retrieve)
// explicitly checks payment_link_config.processor_merchant_id/merchant_id
// against the requestor before returning PaymentLinkNotFound otherwise -
// this is the meaningful cross-merchant check, since it's the one route
// with an actual bearer credential involved. The path-based
// initiate/secure/status routes use MerchantIdAuth(merchant_id) (the
// path's own merchant_id becomes the authenticated identity) plus
// merchant-scoped core queries - there's no separate credential to test
// there, only "does the path's merchant_id/payment_id pair resolve," which
// is covered by Payment Link's own creation/retrieval working correctly.
//
// Route (verified): GET /payment_link/{payment_link_id} - ApiKeyAuth.
let globalState;
let apiKeyM1;
let apiKeyM2;
let paymentLinkId;

describe("OMP Scoping - Payment Link", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      apiKeyM1 = globalState.get("apiKeyM1");
      apiKeyM2 = globalState.get("apiKeyM2");
      paymentLinkId = globalState.get("paymentLinkId");

      if (!paymentLinkId) {
        cy.task(
          "cli_log",
          "Skipping this suite: no paymentLinkId in globalState - the " +
            "PAYMENT_LINK_PA creation step in 00-Setup.cy.js may have failed."
        );
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!paymentLinkId) this.skip();
  });

  it("M2 retrieves M1's payment link - denied", () => {
    cy.ompRetrievePaymentLink(
      paymentLinkId,
      { type: "apiKey", apiKey: apiKeyM2 },
      globalState
    ).then((res) => {
      expect(res.status).to.be.within(400, 499);
    });
  });

  it("M1 retrieves its own payment link (positive)", () => {
    cy.ompRetrievePaymentLink(
      paymentLinkId,
      { type: "apiKey", apiKey: apiKeyM1 },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(200);
      expect(res.body.payment_link_id).to.eq(paymentLinkId);
    });
  });

  it("retrieving a non-existent payment_link_id returns 404", () => {
    cy.ompRetrievePaymentLink(
      "plink_nonexistent_omp_test",
      { type: "apiKey", apiKey: apiKeyM1 },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(404);
    });
  });

  it("invalid credential on payment link retrieve is rejected", () => {
    cy.ompRetrievePaymentLink(
      paymentLinkId,
      { type: "invalid" },
      globalState
    ).then((res) => {
      expect(res.status).to.eq(401);
    });
  });
});
