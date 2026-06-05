import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payout/Utils";

let globalState;
let payoutBody;

describe("[Payout] Recurring", () => {
  let shouldContinue = true;

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      // Synchronously set shouldContinue after globalState is available
      // This ensures the flag is set before any beforeEach checks run
      if (
        !globalState.get("payoutsExecution") ||
        !utils.CONNECTOR_LISTS?.INCLUDE?.PAYOUT_RECURRING?.includes(
          globalState.get("connectorId")
        )
      ) {
        shouldContinue = false;
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  context("Create payout with recurring=true", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create-payout-with-recurring-true", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["RecurringTrue"];

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(payoutBody, data, true, false, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);

      // Verify payout_method_id is returned and non-null for recurring payouts
      cy.wrap(globalState.get("payoutMethodId")).should("exist");
      cy.wrap(globalState.get("payoutMethodId")).should("not.be.null");
      cy.wrap(globalState.get("payoutMethodId")).should("not.eq", "");
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Create payout with recurring=false", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create-payout-with-recurring-false", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["RecurringFalse"];

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(payoutBody, data, true, false, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Create payout without recurring field (defaults to false)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create-payout-with-recurring-omitted", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["RecurringDefault"];

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(payoutBody, data, true, false, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Validation: payout_method_id requires confirm=true", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("attempt-payout-without-confirm-should-fail", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["RecurringInvalidConfirm"];

      if (!utils.should_continue_further(data)) {
        return;
      }

      // Inject real payout_method_id from RecurringTrue to pass deserialization
      // so the confirm=false validation runs and returns the expected error
      data.Request.payout_method_id = globalState.get("payoutMethodId");

      // This test validates that using payout_method_id with confirm=false returns error
      cy.createConfirmPayoutTest(payoutBody, data, false, false, globalState);

      // For error responses, we expect should_continue_further to return false
      // but the test itself should pass (asserting the error)
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  // Entity Type test cases
  context("Create payout with entity_type=Company", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create-payout-with-entity-type-company", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["EntityTypeCompany"];

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(payoutBody, data, true, false, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Create payout with entity_type=Individual", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create-payout-with-entity-type-individual", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["EntityTypeIndividual"];

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(payoutBody, data, true, false, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Create payout with entity_type=NaturalPerson", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create-payout-with-entity-type-natural-person", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["EntityTypeNaturalPerson"];

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(payoutBody, data, true, false, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Create payout with entity_type=NonProfit", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create-payout-with-entity-type-non-profit", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["EntityTypeNonProfit"];

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(payoutBody, data, true, false, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Create payout with entity_type=Personal", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create-payout-with-entity-type-personal", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["EntityTypePersonal"];

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(payoutBody, data, true, false, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Create payout with entity_type=PublicSector", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create-payout-with-entity-type-public-sector", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["EntityTypePublicSector"];

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(payoutBody, data, true, false, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Create payout without entity_type field (defaults to Individual)", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create-payout-with-entity-type-default", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["EntityTypeDefault"];

      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
        return;
      }

      cy.createConfirmPayoutTest(payoutBody, data, true, false, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context("Validation: invalid entity_type should fail", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    beforeEach("reset payoutBody", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create-payout-with-invalid-entity-type-should-fail", () => {
      const data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "bank_transfer_pm"
      ]["sepa_bank_transfer"]["EntityTypeInvalid"];

      if (!utils.should_continue_further(data)) {
        return;
      }

      // This test validates that invalid entity_type returns 400 error
      cy.createConfirmPayoutTest(payoutBody, data, false, false, globalState);

      // For error responses, we expect should_continue_further to return false
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });
});
