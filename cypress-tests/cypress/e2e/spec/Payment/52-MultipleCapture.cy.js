import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Card - Multiple Capture payment flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.MULTIPLE_CAPTURE
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
    "Card - NoThreeDS Multiple Partial Capture payment flow test - Create and Confirm",
    () => {
      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> First Partial Capture -> Second Partial Capture -> Third Partial Capture -> Retrieve Payment after Multiple Captures", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent with manual_multiple", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "manual_multiple",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Payment Methods Call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Payment Methods Call");
            return;
          }
          cy.paymentMethodsCallTest(globalState);
        });

        cy.step("Confirm Payment Intent", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment Intent");
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSManualCapture"];

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

        cy.step("Retrieve Payment after Confirmation", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Confirmation"
            );
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data: confirmData });

          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("First Partial Capture", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: First Partial Capture");
            return;
          }
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["MultipleCapturePartial"];

          cy.captureCallTest(fixtures.captureBody, captureData, globalState);

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Second Partial Capture", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Second Partial Capture");
            return;
          }
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["MultipleCapturePartial"];

          cy.captureCallTest(fixtures.captureBody, captureData, globalState);

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Third Partial Capture", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Third Partial Capture");
            return;
          }
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["MultipleCaptureFinal"];

          cy.captureCallTest(fixtures.captureBody, captureData, globalState);

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Multiple Captures", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Multiple Captures"
            );
            return;
          }
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["MultipleCaptureRetrieve"];

          cy.retrievePaymentCallTest({ globalState, data: captureData });
        });
      });
    }
  );

  context("Card - NoThreeDS Multiple Capture with over-capture attempt", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> First Partial Capture -> Over-capture Attempt", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with manual_multiple", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual_multiple",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment Intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment Intent");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

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

      cy.step("First Partial Capture", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: First Partial Capture");
          return;
        }
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MultipleCapturePartial"];

        cy.captureCallTest(fixtures.captureBody, captureData, globalState);

        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      cy.step("Over-capture Attempt", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Over-capture Attempt");
          return;
        }
        const overcaptureData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["MultipleCaptureOvercapture"];

        cy.captureCallTest(fixtures.captureBody, overcaptureData, globalState);
      });
    });
  });
});
