import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("iDEAL Bank Redirect - Mandates using Payment Method Id flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "iDEAL - Create and Confirm Automatic CIT and MIT payment flow test",
    () => {
      it("customer-create-call-test -> Create iDEAL Payment Intent -> Confirm iDEAL CIT -> retrieve-payment-call-test -> Confirm iDEAL MIT -> retrieve-payment-call-test", () => {
        let shouldContinue = true;

        cy.step("customer-create-call-test", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Create iDEAL Payment Intent", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create iDEAL Payment Intent");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["PaymentIntentOffSession"];

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

        cy.step("Confirm iDEAL CIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm iDEAL CIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["ConfirmCIT"];

          if (!utils.should_continue_further(data)) {
            cy.task(
              "cli_log",
              "Skipping step: Confirm iDEAL CIT (TRIGGER_SKIP)"
            );
            shouldContinue = false;
            return;
          }

          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            10000,
            true,
            "automatic",
            "new_mandate",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle iDEAL redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle iDEAL redirection");
            return;
          }
          const expected_redirection = fixtures.citConfirmBody["return_url"];
          cy.handleBankRedirectRedirection(
            globalState,
            "ideal",
            expected_redirection
          );
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["ConfirmCIT"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm iDEAL MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm iDEAL MIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["ConfirmMIT"];

          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            data,
            5000,
            true,
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["ConfirmMIT"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context(
    "iDEAL - Create and Confirm Manual CIT and MIT payment flow test",
    () => {
      it("Create iDEAL Payment Intent -> Confirm iDEAL CIT -> cit-capture-call-test -> retrieve-payment-call-test -> Confirm iDEAL MIT -> retrieve-payment-call-test", () => {
        let shouldContinue = true;

        cy.step("Create iDEAL Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["PaymentIntentOffSession"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "manual",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm iDEAL CIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm iDEAL CIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["ConfirmCITManual"];

          if (!data || !utils.should_continue_further(data)) {
            cy.task(
              "cli_log",
              "Skipping step: Confirm iDEAL CIT (TRIGGER_SKIP or no config)"
            );
            shouldContinue = false;
            return;
          }

          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            10000,
            true,
            "manual",
            "new_mandate",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle iDEAL redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle iDEAL redirection");
            return;
          }
          const expected_redirection = fixtures.citConfirmBody["return_url"];
          cy.handleBankRedirectRedirection(
            globalState,
            "ideal",
            expected_redirection
          );
        });

        cy.step("cit-capture-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: cit-capture-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["Capture"];

          cy.captureCallTest(fixtures.captureBody, data, 8000, globalState);
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm iDEAL MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm iDEAL MIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["ConfirmMITManual"];

          cy.mitUsingPMId(
            fixtures.pmIdConfirmBody,
            data,
            5000,
            true,
            "manual",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "bank_redirect_pm"
          ]["ConfirmMITManual"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );
});
