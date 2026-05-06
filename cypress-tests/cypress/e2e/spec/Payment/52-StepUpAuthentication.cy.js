import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Step-Up Authentication flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connectorId = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connectorId,
            utils.CONNECTOR_LISTS.INCLUDE.STEP_UP_AUTH
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
    "Card-Step-Up-Auth flow test Create, Confirm with Step-Up, Initiate 3DS and Retrieve",
    () => {
      it("create confirm payment with step-up auth -> initiate 3ds auth -> retrieve payment", () => {
        let shouldContinue = true;

        cy.step("create payment", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["StepUpAuth_Create"];

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

        cy.step("confirm payment with step-up auth", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: confirm payment with step-up auth");
            return;
          }

          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["StepUpAuth_Confirm"];

          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("initiate 3ds authentication", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: initiate 3ds authentication");
            return;
          }

          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["StepUpAuth_Initiate"];

          cy.threeDsAuthenticationCallTest(globalState, data);

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
