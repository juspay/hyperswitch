import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - 3DS Routing Region (UAS) payment flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connectorId = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connectorId,
            utils.CONNECTOR_LISTS.EXCLUDE.THREE_DS
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
    "Card-3DS-UAS-Routing payment flow test Create, Confirm and Retrieve",
    () => {
      it("create confirm payment with 3DS UAS routing -> retrieve payment", () => {
        let shouldContinue = true;

        cy.step("create and confirm payment with 3DS UAS routing", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["three_ds_uas_routing"];

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

  context(
    "Card-3DS-UAS-Routing with manual capture flow test",
    () => {
      it("create confirm capture payment with 3DS UAS routing", () => {
        let shouldContinue = true;

        cy.step("create and confirm payment with manual capture", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["three_ds_uas_routing_manual_capture"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "three_ds",
            "manual",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("capture payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: capture payment");
            return;
          }
          cy.capturePaymentCallTest({ globalState });
        });

        cy.step("retrieve payment after capture", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve payment after capture");
            return;
          }
          cy.retrievePaymentCallTest({ globalState });
        });
      });
    }
  );
});
