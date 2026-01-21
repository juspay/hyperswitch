/**
 * Capitec VRP (Variable & Recurring Payments) Test Suite
 *
 * Based on: Capitec Pay - VRP Test Plan v1.1 (10 July 2025)
 *
 * Test Coverage:
 * 3.1 Once Off Variable Payments (TC1-TC6)
 *   - TC1: Consent Request with different identifiers (Cell, ID, Account)
 *   - TC2: Consent Request Validation
 *   - TC3: Consent Status Request
 *   - TC5: Revoke Consent
 *   - TC6: Action Payment (requires approved consent)
 *
 * 3.2 Variable Recurring Payments (TC1-TC6)
 *   - TC1: Consent Request with recurrence
 *   - TC2: Consent Request Validation (recurrence intervals)
 *   - TC4: Revoke Consent
 *   - TC5: Status Request
 *   - TC6: Action Payment
 *
 * Note: TC4 (Simulate Client Response) requires a real Capitec mobile app
 *
 * Test Users (QA Environment):
 *   - Veronica Fox: Cell 0609603632, ID 8906244547089, Account 2409425506
 *   - Tshepo Moreki: Cell 0609603633, ID 8906241825082, Account 2409425514
 *   - Merchant: CapitecPayTest
 */

import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Capitec VRP Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  // ============================================
  // 3.1 Once Off Variable Payments
  // ============================================

  context("3.1 Once Off Variable Payments", () => {

    // TC1: Consent Request - Cellphone Identifier
    context("TC1: Once-off Consent Request (Cellphone)", () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create-payment-intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-open-banking-consent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["OnceOffConsent"];

        cy.confirmCapitecVrpConsentTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      // TC3: Consent Status Request
      it("sync-consent-status", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["SyncConsent"];

        cy.retrievePaymentCallTest(globalState, data);
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      // TC5: Revoke Consent
      it("revoke-consent", () => {
        cy.revokeMandateCallTest(globalState);
      });
    });

    // TC1: Consent Request - ID Number Identifier
    context("TC1: Once-off Consent Request (ID Number)", () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create-payment-intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-open-banking-consent-id-number", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["OnceOffConsentIdNumber"];

        cy.confirmCapitecVrpConsentTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("sync-consent-status", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["SyncConsent"];

        cy.retrievePaymentCallTest(globalState, data);
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("revoke-consent", () => {
        cy.revokeMandateCallTest(globalState);
      });
    });

    // TC1: Consent Request - Account Number Identifier
    context("TC1: Once-off Consent Request (Account Number)", () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create-payment-intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-open-banking-consent-account-number", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["OnceOffConsentAccountNumber"];

        cy.confirmCapitecVrpConsentTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("sync-consent-status", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["SyncConsent"];

        cy.retrievePaymentCallTest(globalState, data);
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("revoke-consent", () => {
        cy.revokeMandateCallTest(globalState);
      });
    });
  });

  // ============================================
  // 3.2 Variable Recurring Payments
  // ============================================

  context("3.2 Variable Recurring Payments", () => {

    // TC1 & TC2: Recurring Consent Request - Monthly (default)
    context("TC1 & TC2: Recurring Consent Request (Monthly)", () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create-payment-intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-recurring-consent-monthly", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["RecurringConsent"];

        cy.confirmCapitecVrpConsentTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      // TC5: Status Request
      it("sync-consent-status", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["SyncConsent"];

        cy.retrievePaymentCallTest(globalState, data);
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      // TC4: Revoke Consent
      it("revoke-consent", () => {
        cy.revokeMandateCallTest(globalState);
      });
    });

    // TC2: Recurring Consent - Daily Interval
    context("TC2: Recurring Consent (Daily Interval)", () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create-payment-intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-recurring-consent-daily", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["RecurringConsentDaily"];

        cy.confirmCapitecVrpConsentTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("revoke-consent", () => {
        cy.revokeMandateCallTest(globalState);
      });
    });

    // TC2: Recurring Consent - Weekly Interval
    context("TC2: Recurring Consent (Weekly Interval)", () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create-payment-intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-recurring-consent-weekly", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["RecurringConsentWeekly"];

        cy.confirmCapitecVrpConsentTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("revoke-consent", () => {
        cy.revokeMandateCallTest(globalState);
      });
    });

    // TC2: Recurring Consent - Fortnightly Interval
    context("TC2: Recurring Consent (Fortnightly Interval)", () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create-payment-intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-recurring-consent-fortnightly", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["RecurringConsentFortnightly"];

        cy.confirmCapitecVrpConsentTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("revoke-consent", () => {
        cy.revokeMandateCallTest(globalState);
      });
    });

    // TC2: Recurring Consent - Biannually Interval
    context("TC2: Recurring Consent (Biannually Interval)", () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create-payment-intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-recurring-consent-biannually", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["RecurringConsentBiannually"];

        cy.confirmCapitecVrpConsentTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("revoke-consent", () => {
        cy.revokeMandateCallTest(globalState);
      });
    });

    // TC2: Recurring Consent - Annually Interval
    context("TC2: Recurring Consent (Annually Interval)", () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("create-payment-intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("confirm-recurring-consent-annually", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["RecurringConsentAnnually"];

        cy.confirmCapitecVrpConsentTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );
        if (shouldContinue) shouldContinue = utils.should_continue_further(data);
      });

      it("revoke-consent", () => {
        cy.revokeMandateCallTest(globalState);
      });
    });
  });
});
