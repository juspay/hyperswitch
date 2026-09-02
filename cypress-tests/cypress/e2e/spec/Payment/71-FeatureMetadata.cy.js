import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Feature Metadata payment flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connectorId = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connectorId,
            utils.CONNECTOR_LISTS.INCLUDE.FEATURE_METADATA
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

  context(
    "Create payment intent with feature_metadata and confirm payment with feature_metadata",
    () => {
      it("Create Payment Intent with feature_metadata → Create+Confirm Payment with feature_metadata → Retrieve Payment", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent with feature_metadata", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntentWithFeatureMetadata"];

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

        cy.step("Create+Confirm Payment with feature_metadata", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Create+Confirm Payment with feature_metadata"
            );
            return;
          }

          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentWithFeatureMetadata"];

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

        cy.step("Retrieve Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }

          cy.retrievePaymentCallTest({ globalState });
        });
      });
    }
  );
});
