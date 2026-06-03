import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Cancel Post Capture flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Card - NoThreeDS Manual Partial Capture then Cancel Post Capture flow test",
    () => {
      it("Create and Confirm Payment Intent -> Retrieve Payment after Confirmation -> Partial Capture -> Retrieve Payment after Partial Capture -> Cancel Post Capture -> Retrieve Payment after Cancel Post Capture", () => {
        let shouldContinue = true;

        cy.step("Create and Confirm Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "no_three_ds",
            "manual",
            globalState
          );

          if (!utils.should_continue_further(data)) {
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
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Capture Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Partial Capture Payment");
            return;
          }
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialCapture"];

          cy.captureCallTest(
            fixtures.captureBody,
            captureData,
            globalState
          );

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Partial Capture", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Partial Capture"
            );
            return;
          }
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialCapture"];

          cy.retrievePaymentCallTest({ globalState, data: captureData });

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Cancel Post Capture", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Cancel Post Capture");
            return;
          }
          const cancelData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["CancelPostCapture"];

          cy.cancelPostCaptureCallTest(
            fixtures.voidBody,
            cancelData,
            globalState
          );

          if (!utils.should_continue_further(cancelData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Cancel Post Capture", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Cancel Post Capture"
            );
            return;
          }
          const cancelData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["CancelPostCapture"];

          cy.retrievePaymentCallTest({ globalState, data: cancelData });
        });
      });
    }
  );
});
