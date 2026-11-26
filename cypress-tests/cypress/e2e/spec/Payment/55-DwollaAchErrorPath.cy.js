import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("ACH Bank Debit Error Path tests", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (
          shouldIncludeConnector(
            globalState.get("connectorId"),
            CONNECTOR_LISTS.INCLUDE.BANK_DEBIT_ERROR_PATH
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

  context("ACH Bank Debit Create + Confirm Error Path", () => {
    it("Create Payment Intent -> List Payment Methods -> Confirm ACH Bank Debit (expect IR_04) -> Retrieve (skipped)", () => {
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
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
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
      });
    });
  });

  context("ACH Bank Debit Direct Create+Confirm Error Path", () => {
    it("Create+Confirm Payment with ACH Bank Debit directly (expect IR_04)", () => {
      cy.step("Create+Confirm ACH Bank Debit Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "bank_debit_pm"
        ]["AchDirectConfirm"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });
    });
  });

  context(
    "ACH Bank Debit with connector_customer_id in body (negative case)",
    () => {
      it("Create+Confirm Payment with connector_customer_id (expect IR_06 unknown field)", () => {
        cy.step(
          "Create+Confirm ACH Bank Debit with connector_customer_id",
          () => {
            const data = getConnectorDetails(globalState.get("connectorId"))[
              "bank_debit_pm"
            ]["AchWithConnectorCustomerId"];
            cy.createConfirmPaymentTest(
              fixtures.createConfirmPaymentBody,
              data,
              "no_three_ds",
              "automatic",
              globalState
            );
          }
        );
      });
    }
  );
});
