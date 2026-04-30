import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - External 3DS payment flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connectorId = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connectorId,
            utils.CONNECTOR_LISTS.INCLUDE.EXTERNAL_THREE_DS
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

  context(
    "Card-External-3DS payment flow test Create, Confirm and Retrieve",
    () => {
      it("create confirm payment with external 3ds -> retrieve payment", () => {
        let shouldContinue = true;

        cy.step("create and confirm payment with external 3ds", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["external_three_ds"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("retrieve payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve payment");
            return;
          }
          cy.retrievePaymentCallTest({ globalState });
        });
      });
    }
  );
});
