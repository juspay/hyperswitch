import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import * as utils from "../../configs/Payment/Utils";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Paze Wallet - Decrypt payment flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.PAZE_DECRYPT
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

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Paze wallet decrypt payment - Create and Confirm flow test",
    () => {
      it("Create Payment Intent -> Confirm Payment -> Retrieve Payment", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent with Paze wallet", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "wallet_pm"
          ]["PaymentIntent"]("Paze");

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

        cy.step("Confirm Paze Wallet Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "wallet_pm"
          ]["PazeDecrypt"];

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
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "wallet_pm"
          ]["PazeDecrypt"];

          cy.retrievePaymentCallTest({ globalState, data });
        });
      });
    }
  );

  context(
    "Paze wallet decrypt payment - Missing complete_response should error",
    () => {
      it("Confirm Paze payment with missing complete_response should fail", () => {
        cy.step(
          "Create Payment Intent with Paze wallet missing data",
          () => {
            const data = getConnectorDetails(globalState.get("connectorId"))[
              "wallet_pm"
            ]["PazeDecryptInvalid"];

            cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
          }
        );
      });
    }
  );
});
