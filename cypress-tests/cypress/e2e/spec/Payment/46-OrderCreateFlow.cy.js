import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import { CONNECTOR_LISTS, shouldIncludeConnector } from "../../configs/Payment/Utils";

let globalState;

before("seed global state", () => {
  cy.task("getGlobalState").then((state) => {
    globalState = new State(state);
  });
});

after("flush global state", () => {
  cy.task("setGlobalState", globalState.data);
});

describe("Order Create Flow Tests", () => {
  let connector;

  before("seed global state and check connector", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.ORDER_CREATE
          )
        ) {
          skip = true;
          return;
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

  context("Order Create with Auto Capture", () => {
    it("Order Create → Confirm with Card → Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Order Create", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "order_create_pm"
        ]["OrderCreate"];

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

      cy.step("Confirm Payment with Card", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with Card");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "order_create_pm"
        ]["OrderCreateConfirm"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        cy.retrievePaymentCallTest({ globalState });
      });
    });
  });
});
