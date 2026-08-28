import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { getCustomExchange } from "../../../e2e/configs/Payment/Modifiers";
import { connectorDetails } from "../../../e2e/configs/Payment/Commons";

let globalState;

describe("Offer Engine", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Eligible offer is surfaced and applied at confirm", () => {
    it("payment intent create call", () => {
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        connectorDetails.offer_engine.PaymentIntentForOffer,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("payment eligibility check surfaces an eligible offer", () => {
      cy.paymentsOfferEligibilityCheck(
        fixtures.eligibilityCheckBody,
        connectorDetails.offer_engine.OfferEligibilityCheck,
        globalState
      );
    });

    it("confirm call applies the selected offer", () => {
      const offerQuoteId = globalState.get("offerQuoteId");
      expect(offerQuoteId, "offerQuoteId").to.not.be.undefined;

      const confirmData = getCustomExchange({
        Request: {
          ...connectorDetails.offer_engine.ConfirmWithOfferApplied.Request,
          offer_details: {
            offer_quote_ids: [offerQuoteId],
          },
        },
        Response:
          connectorDetails.offer_engine.ConfirmWithOfferApplied.Response,
      });

      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);
    });

    it("applied_offer is reflected on payment retrieve", () => {
      const paymentId = globalState.get("paymentID");
      const baseUrl = globalState.get("baseUrl");
      const apiKey = globalState.get("apiKey");

      cy.request({
        method: "GET",
        url: `${baseUrl}/payments/${paymentId}?force_sync=true`,
        headers: {
          "Content-Type": "application/json",
          "api-key": apiKey,
        },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status, "status_code").to.equal(200);
        expect(response.body.status, "status").to.equal("succeeded");
        expect(response.body.net_amount, "net_amount").to.equal(98000);
        expect(response.body.amount_received, "amount_received").to.equal(
          98000
        );

        expect(response.body, "applied_offer").to.have.property(
          "applied_offer"
        );
        const appliedOffer = response.body.applied_offer;
        expect(appliedOffer, "applied_offer").to.not.be.null;
        // offer_id identifies the underlying offer (stable per offer config),
        // distinct from offer_quote_id which is a per-transaction quote reference
        expect(appliedOffer.offer_id, "applied_offer.offer_id").to.be.a(
          "string"
        ).and.not.be.empty;
        expect(
          appliedOffer.offer_amount,
          "applied_offer.offer_amount"
        ).to.equal(2000);
        expect(appliedOffer.currency, "applied_offer.currency").to.equal("USD");
        expect(
          appliedOffer.offer_engine_merchant_id,
          "applied_offer.offer_engine_merchant_id"
        ).to.be.a("string").and.not.be.empty;
        expect(
          appliedOffer.offer_engine_txn_id,
          "applied_offer.offer_engine_txn_id"
        ).to.be.a("string").and.not.be.empty;
      });
    });
  });

  context("Payment without an offer stays unaffected", () => {
    it("payment intent create call", () => {
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        connectorDetails.offer_engine.PaymentIntentNoOffer,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("confirm call without offer_details leaves applied_offer null", () => {
      cy.confirmCallTest(
        fixtures.confirmBody,
        connectorDetails.offer_engine.ConfirmWithoutOffer,
        true,
        globalState
      );
    });
  });
});
