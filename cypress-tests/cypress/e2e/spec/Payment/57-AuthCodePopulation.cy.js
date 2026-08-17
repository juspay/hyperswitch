import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Card - Auth Code Population test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(connector, CONNECTOR_LISTS.INCLUDE.AUTH_CODE)
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
    "Card-NoThreeDS Auto Capture - auth_code population and persistence",
    () => {
      it("Create+Confirm Payment -> Verify auth_code -> Retrieve Payment -> Verify auth_code persisted", () => {
        let shouldContinue = true;

        cy.step("Create and Confirm Payment (No3DS Auto Capture)", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          ).then((response) => {
            expect(
              response.body.payment_method_data.card.auth_code,
              "auth_code should be populated in create+confirm response"
            ).to.not.be.null;
            expect(
              response.body.payment_method_data.card.auth_code,
              "auth_code should be a non-empty string"
            ).to.be.a("string").and.not.be.empty;
          });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment - verify auth_code persisted", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["AuthCode"];

          cy.retrievePaymentCallTest({ globalState, data }).then((response) => {
            expect(
              response.body.payment_method_data.card.auth_code,
              "auth_code should be persisted in retrieve response"
            ).to.not.be.null;
            expect(
              response.body.payment_method_data.card.auth_code,
              "auth_code should be a non-empty string"
            ).to.be.a("string").and.not.be.empty;
          });
        });
      });
    }
  );

  context(
    "Card-NoThreeDS Manual Capture - auth_code population and persistence through capture",
    () => {
      it("Create+Confirm Payment -> Verify auth_code -> Capture Payment -> Verify auth_code persists -> Retrieve Payment -> Verify auth_code still present", () => {
        let shouldContinue = true;

        cy.step("Create and Confirm Payment (No3DS Manual Capture)", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "no_three_ds",
            "manual",
            globalState
          ).then((response) => {
            expect(
              response.body.payment_method_data.card.auth_code,
              "auth_code should be populated in create+confirm response"
            ).to.not.be.null;
            expect(
              response.body.payment_method_data.card.auth_code,
              "auth_code should be a non-empty string"
            ).to.be.a("string").and.not.be.empty;
          });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step(
          "Capture Payment - verify auth_code persists after capture",
          () => {
            if (!shouldContinue) {
              cy.task("cli_log", "Skipping step: Capture Payment");
              return;
            }
            const data = getConnectorDetails(globalState.get("connectorId"))[
              "card_pm"
            ]["Capture"];

            cy.captureCallTest(fixtures.captureBody, data, globalState).then(
              (response) => {
                expect(
                  response.body.payment_method_data.card.auth_code,
                  "auth_code should persist after capture"
                ).to.not.be.null;
                expect(
                  response.body.payment_method_data.card.auth_code,
                  "auth_code should be a non-empty string"
                ).to.be.a("string").and.not.be.empty;
              }
            );

            if (!utils.should_continue_further(data)) {
              shouldContinue = false;
            }
          }
        );

        cy.step(
          "Retrieve Payment - verify auth_code still present after capture",
          () => {
            if (!shouldContinue) {
              cy.task("cli_log", "Skipping step: Retrieve Payment");
              return;
            }
            const data = getConnectorDetails(globalState.get("connectorId"))[
              "card_pm"
            ]["AuthCode"];

            cy.retrievePaymentCallTest({ globalState, data }).then(
              (response) => {
                expect(
                  response.body.payment_method_data.card.auth_code,
                  "auth_code should be present in retrieve response after capture"
                ).to.not.be.null;
                expect(
                  response.body.payment_method_data.card.auth_code,
                  "auth_code should be a non-empty string"
                ).to.be.a("string").and.not.be.empty;
              }
            );
          }
        );
      });
    }
  );
});
