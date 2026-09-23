import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../configs/Payment/Commons";
import * as utils from "../../configs/Payment/Utils";

let globalState;
let originalCustomerId;
let savedCardFilteringSkip = false;

describe("BIN Based Payment Eligibility via Blocklist Guard", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      originalCustomerId = globalState.get("customerId");
      savedCardFilteringSkip = utils.shouldExcludeConnector(
        globalState.get("connectorId"),
        utils.CONNECTOR_LISTS.EXCLUDE.SAVED_CARD_FILTERING
      );
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  after("restore customer id and flush global state", () => {
    // The saved-card filtering tests below run against a customer created
    // by this spec, so restore the customer id the standard setup chain
    // (02-CustomerCreate) seeded for the specs that follow.
    globalState.set("customerId", originalCustomerId);
    cy.task("setGlobalState", globalState.data);
  });

  context("Setup Phase", () => {
    it("customer create call", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("create saved card payment method with card_isin 411111", () => {
      cy.createPaymentMethodTest(
        globalState,
        connectorDetails.eligibility_api.SavedCard411111
      );
    });

    it("create saved card payment method with card_isin 400005", () => {
      cy.createPaymentMethodTest(
        globalState,
        connectorDetails.eligibility_api.SavedCard400005
      );
    });

    it("create saved card payment method with card_isin 424242", () => {
      cy.createPaymentMethodTest(
        globalState,
        connectorDetails.eligibility_api.SavedCard424242
      );
    });

    it("create saved card payment method with card_isin 555555", () => {
      cy.createPaymentMethodTest(
        globalState,
        connectorDetails.eligibility_api.SavedCard555555
      );
    });

    it("payment intent create call", () => {
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        connectorDetails.eligibility_api.PaymentIntentForBlocklist,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("should enable blocklist guard", () => {
      cy.blocklistToggle(true, globalState);
    });

    it("should create blocklist rule for card_bin 411111", () => {
      cy.blocklistCreateRule(
        fixtures.blocklistCreateBody,
        "411111",
        globalState,
        "card_bin"
      );
    });

    it("should create blocklist rule for extended_card_bin 42424242", () => {
      cy.blocklistCreateRule(
        fixtures.blocklistCreateBody,
        "42424242",
        globalState,
        "extended_card_bin"
      );
    });

    it("should create blocklist rule for generic_card_bin 5555444", () => {
      cy.blocklistCreateRule(
        fixtures.blocklistCreateBody,
        "5555444",
        globalState,
        "generic_card_bin"
      );
    });

    it("should create blocklist rule for generic_card_bin 400005", () => {
      cy.blocklistCreateRule(
        fixtures.blocklistCreateBody,
        "400005",
        globalState,
        "generic_card_bin"
      );
    });
  });

  context("Eligibility API Deny Tests", () => {
    it("should deny 8-digit card_bin matched by 6-digit card_bin entry prefix", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBinBody,
        connectorDetails.eligibility_api.EightDigitPrefixMatch,
        globalState
      );
    });

    it("should deny 7-digit card_bin matched by 6-digit card_bin entry prefix", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBinBody,
        connectorDetails.eligibility_api.BinOnlyBlocked,
        globalState
      );
    });

    it("should deny card_bin matched by extended_card_bin entry", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBinBody,
        connectorDetails.eligibility_api.ExtendedBinBlocked,
        globalState
      );
    });

    it("should deny card_bin matched by generic_card_bin entry", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBinBody,
        connectorDetails.eligibility_api.GenericBinBlocked,
        globalState
      );
    });

    it("should deny 10-digit card_bin with blocked 6-digit prefix", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBinBody,
        connectorDetails.eligibility_api.TenDigitBinPrefixBlocked,
        globalState
      );
    });
  });

  context("Eligibility API Allow Tests", () => {
    it("should allow 6-digit card_bin not probed by 8-digit extended_card_bin entry", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBinBody,
        connectorDetails.eligibility_api.SixDigitNotBlockedByEightDigitEntry,
        globalState
      );
    });

    it("should allow 6-digit card_bin not probed by 7-digit generic_card_bin entry", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBinBody,
        connectorDetails.eligibility_api.SixDigitNotBlockedBySevenDigitEntry,
        globalState
      );
    });

    it("should allow unlisted 6-digit card_bin", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBinBody,
        connectorDetails.eligibility_api.UnlistedBinAllowed,
        globalState
      );
    });

    it("should allow full card with unlisted BIN", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBody,
        connectorDetails.eligibility_api.UnlistedFullCardAllowed,
        globalState
      );
    });
  });

  context("Eligibility API Malformed BIN Error Tests", () => {
    it("should return IR_06 for 5-digit card_bin", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBinBody,
        connectorDetails.eligibility_api.MalformedBinError5Digit,
        globalState
      );
    });

    it("should return IR_06 for 11-digit card_bin", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBinBody,
        connectorDetails.eligibility_api.MalformedBinError11Digit,
        globalState
      );
    });

    it("should return IR_06 for non-digit card_bin", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBinBody,
        connectorDetails.eligibility_api.MalformedBinErrorNonDigit,
        globalState
      );
    });
  });

  context("Eligibility API Guard Disabled Tests", () => {
    it("should disable blocklist guard", () => {
      cy.blocklistToggle(false, globalState);
    });

    it("should allow previously blocked card_bin when guard is disabled", () => {
      cy.paymentsEligibilityCheck(
        fixtures.eligibilityCheckBinBody,
        connectorDetails.eligibility_api.GuardDisabledAllowsBlockedBin,
        globalState
      );
    });
  });

  context("Saved Card Filtering Tests", () => {
    it("should re-enable blocklist guard", () => {
      cy.blocklistToggle(true, globalState);
    });

    it("should filter blocklisted saved cards from client payment methods list with guard on", function () {
      if (savedCardFilteringSkip) {
        this.skip();
      }
      cy.paymentsClientListCallTest(
        connectorDetails.eligibility_api.SavedCardFilteringGuardOn,
        globalState
      );
    });

    it("should disable blocklist guard", () => {
      cy.blocklistToggle(false, globalState);
    });

    it("should return all saved cards in client payment methods list with guard off", function () {
      if (savedCardFilteringSkip) {
        this.skip();
      }
      cy.paymentsClientListCallTest(
        connectorDetails.eligibility_api.SavedCardFilteringGuardOff,
        globalState
      );
    });

    it("should re-enable blocklist guard", () => {
      cy.blocklistToggle(true, globalState);
    });

    it("should delete blocklist rule for generic_card_bin 400005", () => {
      cy.blocklistDeleteRule("generic_card_bin", "400005", globalState);
    });

    it("should return deleted entry saved card in client payment methods list", function () {
      if (savedCardFilteringSkip) {
        this.skip();
      }
      cy.paymentsClientListCallTest(
        connectorDetails.eligibility_api.DeleteEntryCardReappears,
        globalState
      );
    });
  });

  context("Cleanup", () => {
    it("should delete blocklist rule for card_bin 411111", () => {
      cy.blocklistDeleteRule("card_bin", "411111", globalState);
    });

    it("should delete blocklist rule for extended_card_bin 42424242", () => {
      cy.blocklistDeleteRule("extended_card_bin", "42424242", globalState);
    });

    it("should delete blocklist rule for generic_card_bin 5555444", () => {
      cy.blocklistDeleteRule("generic_card_bin", "5555444", globalState);
    });

    it("should disable blocklist guard", () => {
      cy.blocklistToggle(false, globalState);
    });
  });
});
