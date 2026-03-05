import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import step from "../../../utils/customStep";

let globalState;

describe("Card - NoThreeDS Manual payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card - void payment in Requires_capture state flow test", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Void Payment without Capture", () => {
      let shouldContinue = true;

<<<<<<< Updated upstream
      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
=======
      step("Create Payment Intent", shouldContinue, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
          "PaymentIntent"
        ];
>>>>>>> Stashed changes

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
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

<<<<<<< Updated upstream
      cy.step("Retrieve Payment after Confirmation", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Confirmation"
          );
          return;
        }

=======
      step("Retrieve Payment after Confirmation", shouldContinue, () => {
>>>>>>> Stashed changes
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });

<<<<<<< Updated upstream
        cy.task(
          "cli_log",
          "Completed step: Retrieve Payment after Confirmation"
        );
=======
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
>>>>>>> Stashed changes
      });

      step("Void Payment without Capture", shouldContinue, () => {
        const voidData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["VoidAfterConfirm"];

        cy.voidCallTest(fixtures.voidBody, voidData, globalState);
      });
    });
  });

  context(
    "Card - void payment in Requires_payment_method state flow test",
    () => {
      it("Create Payment Intent -> Payment Methods Call -> Void Payment without Confirmation", () => {
        let shouldContinue = true;

<<<<<<< Updated upstream
        cy.step("Create Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];
=======
        step("Create Payment Intent", shouldContinue, () => {
          const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
            "PaymentIntent"
          ];
>>>>>>> Stashed changes

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "manual",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        step("Payment Methods Call", shouldContinue, () => {
          cy.paymentMethodsCallTest(globalState);
        });

<<<<<<< Updated upstream
        cy.step("Void Payment without Confirmation", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Void Payment without Confirmation"
            );
            return;
          }

=======
        step("Void Payment without Confirmation", shouldContinue, () => {
>>>>>>> Stashed changes
          const voidData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Void"];

          cy.voidCallTest(fixtures.voidBody, voidData, globalState);
<<<<<<< Updated upstream

          cy.task(
            "cli_log",
            "Completed step: Void Payment without Confirmation"
          );
=======
>>>>>>> Stashed changes
        });
      });
    }
  );

  context("Card - void payment in success state flow test", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Void Payment after Confirmation", () => {
      let shouldContinue = true;

<<<<<<< Updated upstream
      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
=======
      step("Create Payment Intent", shouldContinue, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
          "PaymentIntent"
        ];
>>>>>>> Stashed changes

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          false,
          globalState
        );

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

<<<<<<< Updated upstream
      cy.step("Retrieve Payment after Confirmation", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Confirmation"
          );
          return;
        }

=======
      step("Retrieve Payment after Confirmation", shouldContinue, () => {
>>>>>>> Stashed changes
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });

<<<<<<< Updated upstream
        cy.task(
          "cli_log",
          "Completed step: Retrieve Payment after Confirmation"
        );
=======
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
>>>>>>> Stashed changes
      });

      step("Void Payment after Confirmation", shouldContinue, () => {
        const voidData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["VoidAfterConfirm"];

        cy.voidCallTest(fixtures.voidBody, voidData, globalState);
      });
    });
  });
});
