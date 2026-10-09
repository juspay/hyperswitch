import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../../e2e/configs/Payment/Commons";
import getConnectorDetails from "../../configs/Payment/Utils";

let globalState;

describe("Offer Engine", () => {
  before("seed global state and verify Offer Engine connectivity", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        return cy.setMerchantOfferEngineConfig(globalState);
      })
      .then(() => {
        // Scoped per merchant (processor_merchant_id), since the untargeted
        // default is "none"/false for all three keys. Only this spec's
        // merchant needs offers enabled, so this lives here rather than in
        // 01-AccountCreate.cy.js's common setup.
        cy.createSuperpositionOverrides(
          globalState,
          {
            "offer_engine.enabled": true,
            "offer_engine.credential_source": "merchant",
            "payments.should_perform_eligibility": true,
          },
          { processor_merchant_id: globalState.get("merchantId") }
        );
      })
      .then(() => {
        return cy.offerEngineConnectivityCheck(globalState);
      })
      .then((reachable) => {
        // /offer_engine/connectivity always reports unreachable when
        // offer_engine.credential_source is "merchant", since that check has
        // no merchant context to resolve credentials against. Fall back to a
        // merchant-scoped signal: does this run's merchant have its own
        // offer_engine_config set (via setMerchantOfferEngineConfig above).
        if (reachable) {
          return cy.wrap(true);
        }

        return cy.offerEngineMerchantConfiguredCheck(globalState);
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
    // Clear so a stale quote id can never leak into a later spec/run via
    // persisted globalState if some future eligibility call here ever
    // returns zero eligible offers (the "set only if present" pattern in
    // paymentsOfferEligibilityCheck would otherwise just leave the old
    // value in place rather than failing loudly).
    globalState.set("offerQuoteId", undefined);
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
      // Clear before each cycle so a stale quote id from a previous
      // context can never silently be reused if this call were to return
      // zero eligible offers.
      globalState.set("offerQuoteId", undefined);
      cy.paymentsOfferEligibilityCheck(
        fixtures.eligibilityCheckBody,
        getConnectorDetails(globalState.get("connectorId")).offer_engine
          .OfferEligibilityCheck,
        globalState
      );
    });

    it("confirm call applies the selected offer", () => {
      const offerEngineDetails = getConnectorDetails(
        globalState.get("connectorId")
      ).offer_engine;
      cy.confirmCallTest(
        fixtures.confirmBody,
        offerEngineDetails.ConfirmWithOfferApplied,
        true,
        globalState
      );
    });

    it("applied_offer is reflected on payment retrieve", () => {
      const offerEngineDetails = getConnectorDetails(
        globalState.get("connectorId")
      ).offer_engine;
      cy.retrievePaymentCallTest({
        globalState,
        data: offerEngineDetails.AppliedOfferOnRetrieve,
        expectedIntentStatus:
          offerEngineDetails.AppliedOfferOnRetrieve.Response.body.status,
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
      const offerEngineDetails = getConnectorDetails(
        globalState.get("connectorId")
      ).offer_engine;
      cy.confirmCallTest(
        fixtures.confirmBody,
        offerEngineDetails.ConfirmWithoutOffer,
        true,
        globalState
      );
    });
  });

  // PR #13766 secondary change: when offers are enabled, the payment-method-
  // list response should signal the SDK to run eligibility before confirm
  // (sdk_next_action.next_action: eligibility_check,
  // should_block_confirm: true) rather than going straight to confirm. This
  // is the actual gating mechanism the SDK relies on — 54-OfferEngine.cy.js's
  // other contexts call /eligibility directly and never exercise this signal.
  context(
    "Payment method list signals eligibility check when offers are enabled",
    () => {
      it("payment intent create call", () => {
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          connectorDetails.offer_engine.PaymentIntentForOffer,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("payment method list signals an eligibility check is required", () => {
        cy.paymentMethodListSdkNextActionCheck(globalState, {
          next_action: "eligibility_check",
          should_block_confirm: true,
        });
      });
    }
  );

  // PR #13850: confirm-time /apply reads the card straight off
  // payment_data.payment_method_data. For a saved-card confirm that's still
  // the CardToken variant at that point (the real card is only dereferenced
  // later, in the connector-call phase), so /apply used to see null
  // card_bin/card_network/card_alias. This exercises exactly that path: a
  // second payment confirmed via payment_token (not raw PAN) with an offer
  // applied.
  context("Offer applies correctly on a saved-card confirm", () => {
    it("create customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("save a card via a plain create+confirm payment", () => {
      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        getConnectorDetails(globalState.get("connectorId"))["card_pm"][
          "SaveCardUseNo3DSAutoCapture"
        ],
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("list customer payment methods", () => {
      cy.listCustomerPMCallTest(globalState);
    });

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
      // Clear before each cycle so a stale quote id from a previous
      // context can never silently be reused if this call were to return
      // zero eligible offers.
      globalState.set("offerQuoteId", undefined);
      cy.paymentsOfferEligibilityCheck(
        fixtures.eligibilityCheckBody,
        getConnectorDetails(globalState.get("connectorId")).offer_engine
          .OfferEligibilityCheck,
        globalState
      );
    });

    it("saved-card confirm call applies the selected offer", () => {
      const saveCardBody = Cypress._.cloneDeep(fixtures.saveCardConfirmBody);
      // Routed through getConnectorDetails() merge (unlike the plain-card
      // flow above, which uses the shared Commons default directly) so
      // connectors that can't complete a saved-card confirm at all (e.g.
      // ilixium -- no repeat_payment support) can override just this key.
      const offerEngineDetails = getConnectorDetails(
        globalState.get("connectorId")
      ).offer_engine;
      cy.saveCardConfirmCallTest(
        saveCardBody,
        offerEngineDetails.ConfirmWithOfferAppliedSavedCard,
        globalState
      );
    });

    it("applied_offer is reflected on payment retrieve", () => {
      const offerEngineDetails = getConnectorDetails(
        globalState.get("connectorId")
      ).offer_engine;
      const savedCardRetrieve =
        offerEngineDetails.AppliedOfferOnRetrieveSavedCard;
      cy.retrievePaymentCallTest({
        globalState,
        data: savedCardRetrieve,
        expectedIntentStatus: savedCardRetrieve.Response.body.status,
      });
    });
  });

  // PR #13766: once a card has availed an offer, Offer Engine blocks that
  // exact card (via a PAN-free card_alias fingerprint) from availing it
  // again, independent of which customer/payment uses it.
  //
  // Commented out for now: this needs a dedicated once-per-card offer
  // (the PR's own testing used offer code HSVELO1, counter
  // CARD_IDENTIFIER = MAX 1) rather than TESTHS, since TESTHS has no
  // redemption cap and reusing it here would collide with the "saved-card"
  // context above (which also uses this same card and needs the offer to
  // still be applicable). HSVELO1 already exists on qaoffers but is
  // currently Paused, and neither of us has access to Offer Engine's
  // "Custom Rule Configuration" to build an equivalent counter from
  // scratch. Re-enable once HSVELO1 is activated.
  //
  // context("Once-per-card offer velocity blocks reuse of the same card", () => {
  //   it("payment intent create call", () => {
  //     cy.createPaymentIntentTest(
  //       fixtures.createPaymentBody,
  //       connectorDetails.offer_engine.PaymentIntentForOffer,
  //       "no_three_ds",
  //       "automatic",
  //       globalState
  //     );
  //   });
  //
  //   it("eligibility check no longer surfaces an offer for the already-used card", () => {
  //     cy.paymentsOfferEligibilityCheck(
  //       fixtures.eligibilityCheckBody,
  //       connectorDetails.offer_engine.VelocityEligibilityCheckSecondUse,
  //       globalState
  //     );
  //   });
  // });
});
