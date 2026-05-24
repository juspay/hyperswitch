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
    it("create payment intent and confirm with valid sdk authorization", () => {
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
          cy.task("cli_log", "Skipping step: Confirm with valid SDK Authorization");
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
    it("create payment intent and confirm with invalid client_session_id", () => {
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

        const invalidConfirmData = {
          ...confirmData,
          ResponseCustom: confirmData.ResponseCustom || confirmData.Response,
        };

        cy.confirmWithSdkAuthTest(
          fixtures.confirmBody,
          invalidConfirmData,
          true,
          globalState,
          "invalid_session"
        );
      });
    });
  });

  context("Missing Client Session - Confirm without CSI (legacy fallback)", () => {
    it("create payment intent and confirm without client_session_id", () => {
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

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ClientSessionValidConfirm"];

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

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ClientSessionValidConfirm"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Replay Client Session - Confirm with old CSI after update", () => {
    it("create, update, confirm with old CSI expect 401, confirm with new CSI expect 200", () => {
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

      cy.step("Save old SDK Authorization", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Save old SDK Authorization");
          return;
        }

        cy.wrap(null).then(() => {
          const oldSdkAuth = globalState.get("sdkAuthorization");
          globalState.set("oldSdkAuthorization", oldSdkAuth);
          cy.task("cli_log", `Saved old sdk_authorization`);
        });
      });

      cy.step("Update Payment Intent - triggers session recreation", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Update Payment Intent"
          );
          return;
        }

        const updateData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ClientSessionUpdatePayment"];

        cy.updatePaymentIntentTest({}, updateData, globalState);

        if (!utils.should_continue_further(updateData)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm with old CSI - expect 401", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm with old CSI");
          return;
        }

        const replayData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ClientSessionReplayConfirm"];

        const replayConfirmData = {
          ...replayData,
          ResponseCustom: replayData.ResponseCustom || replayData.Response,
        };

        cy.confirmWithSdkAuthTest(
          fixtures.confirmBody,
          replayConfirmData,
          true,
          globalState,
          globalState.get("oldSdkAuthorization")
        );
      });

      cy.step("Confirm with new CSI - expect 200", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm with new CSI");
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

  context("Toggle client_session_validation_enabled - disabled allows invalid CSI", () => {
    it("disable validation, confirm with invalid CSI succeeds, re-enable validation, confirm with invalid CSI fails", () => {
      let shouldContinue = true;

      cy.step("Disable client_session_validation_enabled", () => {
        cy.setConfigs(
          globalState,
          "client_session_validation_enabled",
          "false",
          "UPDATE"
        );
      });

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

      cy.step("Confirm with invalid CSI - should succeed when validation disabled", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm with invalid CSI (validation disabled)"
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
          globalState,
          "invalid_session"
        );

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      cy.step("Re-enable client_session_validation_enabled", () => {
        cy.setConfigs(
          globalState,
          "client_session_validation_enabled",
          "true",
          "UPDATE"
        );
      });

      cy.step("Create another Payment Intent", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create another Payment Intent"
          );
          return;
        }

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

      cy.step("Confirm with invalid CSI - should fail when validation enabled", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm with invalid CSI (validation enabled)"
          );
          return;
        }

        const invalidData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ClientSessionInvalidConfirm"];

        const invalidConfirmData = {
          ...invalidData,
          ResponseCustom: invalidData.ResponseCustom || invalidData.Response,
        };

        cy.confirmWithSdkAuthTest(
          fixtures.confirmBody,
          invalidConfirmData,
          true,
          globalState,
          "invalid_session"
        );
      });
    });
  });
});
