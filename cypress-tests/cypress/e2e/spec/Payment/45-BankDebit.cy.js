import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Bank Debit tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.BANK_DEBIT
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

  let shouldContinue = true;

  beforeEach(function () {
    if (!shouldContinue) {
      this.skip();
    }
  });

  it("Create Payment Intent -> List Merchant Payment Methods -> Confirm SEPA Bank Debit -> Retrieve Payment", () => {
    let shouldContinue = true;

    cy.step("Create Payment Intent for SEPA", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["PaymentIntent"]("Sepa");
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
      }
    });

    cy.step("List Merchant Payment Methods", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
        return;
      }
      cy.paymentMethodsCallTest(globalState);
    });

    cy.step("Confirm SEPA Bank Debit", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Confirm SEPA Bank Debit");
        return;
      }
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["Sepa"];
      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);
      if (!utils.should_continue_further(confirmData)) {
        shouldContinue = false;
      }
    });

    cy.step("Retrieve Payment", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Retrieve Payment");
        return;
      }
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["Sepa"];
      cy.retrievePaymentCallTest({ globalState, data: confirmData });
      if (!utils.should_continue_further(confirmData)) {
        shouldContinue = false;
      }
    });
  });

  it("Create Payment Intent -> List Merchant Payment Methods -> Confirm ACH Bank Debit -> Retrieve Payment", () => {
    let shouldContinue = true;

    cy.step("Create Payment Intent for ACH", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["PaymentIntent"]("Ach");
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
      }
    });

    cy.step("List Merchant Payment Methods", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
        return;
      }
      cy.paymentMethodsCallTest(globalState);
    });

    cy.step("Confirm ACH Bank Debit", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Confirm ACH Bank Debit");
        return;
      }
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["Ach"];
      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);
      if (!utils.should_continue_further(confirmData)) {
        shouldContinue = false;
      }
    });

    cy.step("Retrieve Payment", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Retrieve Payment");
        return;
      }
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["Ach"];
      cy.retrievePaymentCallTest({ globalState, data: confirmData });
      if (!utils.should_continue_further(confirmData)) {
        shouldContinue = false;
      }
    });
  });

  it("Create Payment Intent -> List Merchant Payment Methods -> Confirm BECS Bank Debit -> Retrieve Payment", () => {
    let shouldContinue = true;

    cy.step("Create Payment Intent for BECS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["PaymentIntent"]("Becs");
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
      }
    });

    cy.step("List Merchant Payment Methods", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
        return;
      }
      cy.paymentMethodsCallTest(globalState);
    });

    cy.step("Confirm BECS Bank Debit", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Confirm BECS Bank Debit");
        return;
      }
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["Becs"];
      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);
      if (!utils.should_continue_further(confirmData)) {
        shouldContinue = false;
      }
    });

    cy.step("Retrieve Payment", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Retrieve Payment");
        return;
      }
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["Becs"];
      cy.retrievePaymentCallTest({ globalState, data: confirmData });
      if (!utils.should_continue_further(confirmData)) {
        shouldContinue = false;
      }
    });
  });

  it("Create Payment Intent -> List Merchant Payment Methods -> Confirm BACS Bank Debit -> Retrieve Payment", () => {
    let shouldContinue = true;

    cy.step("Create Payment Intent for BACS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["PaymentIntent"]("Bacs");
      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (!utils.should_continue_further(data)) {
        shouldContinue = false;
      }
    });

    cy.step("List Merchant Payment Methods", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: List Merchant Payment Methods");
        return;
      }
      cy.paymentMethodsCallTest(globalState);
    });

    cy.step("Confirm BACS Bank Debit", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Confirm BACS Bank Debit");
        return;
      }
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["Bacs"];
      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);
      if (!utils.should_continue_further(confirmData)) {
        shouldContinue = false;
      }
    });

    cy.step("Retrieve Payment", () => {
      if (!shouldContinue) {
        cy.task("cli_log", "Skipping step: Retrieve Payment");
        return;
      }
      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "bank_debit_pm"
      ]["Bacs"];
      cy.retrievePaymentCallTest({ globalState, data: confirmData });
      if (!utils.should_continue_further(confirmData)) {
        shouldContinue = false;
      }
    });
  });

  it("create-customer-call-test", () => {
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
  });

  it("sepa-bank-debit-mandate-cit-test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_debit_pm"
    ]["SepaMandate"];

    cy.citForMandatesCallTest(
      fixtures.citConfirmBody,
      data,
      6000,
      true,
      "automatic",
      "new_mandate",
      globalState
    );

    if (shouldContinue) shouldContinue = utils.should_continue_further(data);
  });

  it("retrieve-mandate-call-test", () => {
    cy.mandateGETCallTest(globalState);
  });

  it("sepa-bank-debit-mandate-mit-test", () => {
    const data = getConnectorDetails(globalState.get("connectorId"))[
      "bank_debit_pm"
    ]["BankdebitMIT"];

    cy.mitForMandatesCallTest(
      fixtures.mitConfirmBody,
      data,
      6000,
      true,
      "automatic",
      globalState
    );

    if (shouldContinue) shouldContinue = utils.should_continue_further(data);
  });

  it("list-mandate-call-test", () => {
    cy.listMandateCallTest(globalState);
  });

  it("revoke-mandate-call-test", () => {
    cy.revokeMandateCallTest(globalState);
  });

  it("bacs-create-customer-call-test", () => {
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
  });
});
