import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
  should_continue_further,
} from "../../configs/Payment/Utils";

let globalState;

describe("Bank Debit Mandate tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.BANK_DEBIT_MANDATE
          )
        ) {
          skip = true;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("SEPA Bank Debit Mandate CIT and MIT flow test", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-customer-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("sepa-bank-debit-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["Sepa"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("retrieve-mandate-call-test", () => {
      cy.mandateGETCallTest(globalState);
    });

    it("sepa-bank-debit-mandate-mit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["BankDebitMandate"];

      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        data,
        6000,
        true,
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });

    it("list-mandate-call-test", () => {
      cy.listMandateCallTest(globalState);
    });

    it("revoke-mandate-call-test", () => {
      cy.revokeMandateCallTest(globalState);
    });
  });

  context("BACS Bank Debit Mandate CIT flow test", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-customer-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("bacs-bank-debit-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["Bacs"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });
  });

  context("ACH Bank Debit Mandate CIT flow test", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-customer-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("ach-bank-debit-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["Ach"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });
  });

  context("BECS Bank Debit Mandate CIT flow test", () => {
    let shouldContinue = true;

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-customer-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("becs-bank-debit-mandate-cit-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["Becs"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6000,
        true,
        "automatic",
        "new_mandate",
        globalState
      );

      if (shouldContinue) shouldContinue = should_continue_further(data);
    });
  });
});
