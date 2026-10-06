import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let connector;
let globalState;

describe("Card - Order Details payment flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        // Skip the test if the connector is not in the inclusion list
        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.ORDER_DETAILS
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

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Create and confirm payment with single order_details item", () => {
    it("Create and Confirm Payment with order_details -> Retrieve Payment with order_details", () => {
      let shouldContinue = true;

      cy.step("Create and Confirm Payment with order_details", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["OrderDetails"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment with order_details", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment with order_details"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["OrderDetails"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context(
    "Create and confirm payment with multiple order_details items",
    () => {
      it("Create and Confirm Payment with multiple order_details -> Retrieve Payment with order_details", () => {
        let shouldContinue = true;

        cy.step(
          "Create and Confirm Payment with multiple order_details",
          () => {
            const data = getConnectorDetails(globalState.get("connectorId"))[
              "card_pm"
            ]["OrderDetailsMultipleItems"];

            cy.createConfirmPaymentTest(
              fixtures.createConfirmPaymentBody,
              data,
              "no_three_ds",
              "automatic",
              globalState
            );

            if (!utils.should_continue_further(data)) {
              shouldContinue = false;
            }
          }
        );

        cy.step("Retrieve Payment with multiple order_details", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment with multiple order_details"
            );
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["OrderDetailsMultipleItems"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context(
    "Create and confirm payment with missing required order_details field",
    () => {
      it("Create and Confirm Payment with missing product_name -> Expect validation error IR_06", () => {
        cy.step("Create and Confirm Payment with missing product_name", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["OrderDetailsMissingProductName"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );
        });
      });
    }
  );
});
