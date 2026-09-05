import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../../e2e/configs/Payment/Commons";

let globalState;

describe("Offer Engine", () => {
  before("seed global state and verify Offer Engine connectivity", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        return cy.offerEngineConnectivityCheck(globalState);
      })
      .then((reachable) => {
        if (!reachable) {
          cy.task(
            "cli_log",
            "Offer Engine is not reachable/enabled in this environment, skipping Offer Engine spec"
          );
          skip = true;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
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
      cy.confirmCallTest(
        fixtures.confirmBody,
        connectorDetails.offer_engine.ConfirmWithOfferApplied,
        true,
        globalState
      );
    });

    it("applied_offer is reflected on payment retrieve", () => {
      cy.retrievePaymentCallTest({
        globalState,
        data: connectorDetails.offer_engine.AppliedOfferOnRetrieve,
        expectedIntentStatus: "succeeded",
      });
    });
  });

  context("Payment without an offer stays unaffected", () => {
    it("payment intent create call", () => {
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        connectorDetails.offer_engine.PaymentIntentForOffer,
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
