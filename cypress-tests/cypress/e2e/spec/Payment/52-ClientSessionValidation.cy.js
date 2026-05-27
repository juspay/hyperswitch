import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Client Session Validation", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Valid Client Session - Confirm with SDK Authorization", () => {
    it("Create Payment Intent -> Confirm with valid SDK Authorization -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
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

      cy.step("Confirm with valid SDK Authorization", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm with valid SDK Authorization"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ClientSessionValidConfirm"];
        cy.confirmWithSdkAuthTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ClientSessionValidConfirm"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Invalid Client Session - Confirm with tampered CSI", () => {
    it("Create Payment Intent -> Confirm with invalid client_session_id - expect 401", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
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

      cy.step("Confirm with invalid client_session_id - expect 401", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm with invalid client_session_id"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ClientSessionInvalidConfirm"];
        cy.confirmWithSdkAuthTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState,
          "invalid_session"
        );
      });
    });
  });

  context(
    "Missing Client Session - Confirm without CSI (legacy fallback)",
    () => {
      it("Create Payment Intent -> Confirm without client_session_id -> Retrieve Payment", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];
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

        cy.step("Confirm without client_session_id - legacy fallback", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Confirm without client_session_id"
            );
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["ClientSessionValidConfirm"];
          cy.confirmWithSdkAuthTest(
            fixtures.confirmBody,
            confirmData,
            true,
            globalState,
            "missing_session"
          );
          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["ClientSessionValidConfirm"];
          cy.retrievePaymentCallTest({ globalState, data: confirmData });
        });
      });
    }
  );
});
