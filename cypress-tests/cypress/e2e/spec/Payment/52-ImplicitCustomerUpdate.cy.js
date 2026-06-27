import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Implicit Customer Update flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    if (globalState && globalState.data) {
      cy.task("setGlobalState", globalState.data);
    }
  });

  context(
    "Create customer, confirm payment with inline customer update, verify customer record updated",
    () => {
      it("Create Customer -> Retrieve Baseline -> Create+Confirm Payment with updated customer fields -> Verify Customer Updated", () => {
        let shouldContinue = true;

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Retrieve Baseline Customer", () => {
          cy.customerRetrieveCall(globalState);
        });

        cy.step("Create+Confirm Payment with updated customer fields", () => {
          const connectorId = globalState.get("connectorId");
          const data =
            getConnectorDetails(connectorId)["card_pm"][
              "ImplicitCustomerUpdate"
            ];

          cy.createConfirmPaymentTest(
            JSON.parse(JSON.stringify(fixtures.createConfirmPaymentBody)),
            data,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Verify Customer Record Updated", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Verify Customer Record Updated");
            return;
          }

          cy.customerRetrieveAndAssertCall(globalState, {
            email: "updated@example.com",
            name: "Updated Name",
            phone: "888888888",
            phone_country_code: "+1",
          });
        });
      });
    }
  );

  context(
    "Create customer, confirm payment with partial inline customer update, verify only specified fields changed",
    () => {
      it("Create Customer -> Confirm Payment with partial update -> Verify only email and name changed", () => {
        let shouldContinue = true;

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Retrieve Baseline Customer", () => {
          cy.customerRetrieveCall(globalState);
        });

        cy.step("Create+Confirm Payment with partial customer fields", () => {
          const connectorId = globalState.get("connectorId");
          const data =
            getConnectorDetails(connectorId)["card_pm"][
              "ImplicitCustomerUpdatePartial"
            ];

          cy.createConfirmPaymentTest(
            JSON.parse(JSON.stringify(fixtures.createConfirmPaymentBody)),
            data,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Verify Partial Customer Record Updated", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Verify Partial Customer Record Updated"
            );
            return;
          }

          cy.customerRetrieveAndAssertCall(globalState, {
            email: "partial@example.com",
            name: "Partial Name",
            phone: "999999999",
            phone_country_code: "+65",
          });
        });
      });
    }
  );
});
