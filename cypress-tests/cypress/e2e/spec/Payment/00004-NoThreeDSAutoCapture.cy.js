import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import step from "../../../utils/customStep";

let globalState;

describe("Card - NoThreeDS payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Card-NoThreeDS payment flow test Create and confirm", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment -> Retrieve Payment", () => {
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
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

      step("Confirm Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

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

      step("Retrieve Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });

  context("Card-NoThreeDS payment flow test Create+Confirm", () => {
    it("Create and Confirm Payment -> Retrieve Payment", () => {
      let shouldContinue = true;

<<<<<<< Updated upstream
      cy.step("Create and Confirm Payment", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
=======
      step("Create and Confirm Payment", shouldContinue, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
          "No3DSAutoCapture"
        ];
>>>>>>> Stashed changes

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

<<<<<<< Updated upstream
      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
=======
      step("Retrieve Payment", shouldContinue, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
          "No3DSAutoCapture"
        ];
>>>>>>> Stashed changes

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Card-NoThreeDS payment with shipping cost", () => {
    it("Create Payment Intent with shipping cost -> Payment Methods Call -> Confirm Payment with shipping cost -> Retrieve Payment with shipping cost", () => {
      let shouldContinue = true;

<<<<<<< Updated upstream
      cy.step("Create Payment Intent with shipping cost", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentWithShippingCost"];
=======
      step("Create Payment Intent with shipping cost", shouldContinue, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
          "PaymentIntentWithShippingCost"
        ];
>>>>>>> Stashed changes

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
<<<<<<< Updated upstream

        cy.task(
          "cli_log",
          "Completed step: Create Payment Intent with shipping cost"
        );
=======
>>>>>>> Stashed changes
      });

      step("Payment Methods Call", shouldContinue, () => {
        cy.paymentMethodsCallTest(globalState);
      });

<<<<<<< Updated upstream
      cy.step("Confirm Payment with shipping cost", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment with shipping cost"
          );
          return;
        }

=======
      step("Confirm Payment with shipping cost", shouldContinue, () => {
>>>>>>> Stashed changes
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentConfirmWithShippingCost"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
<<<<<<< Updated upstream

        cy.task(
          "cli_log",
          "Completed step: Confirm Payment with shipping cost"
        );
      });

      cy.step("Retrieve Payment with shipping cost", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment with shipping cost"
          );
          return;
        }

=======
      });

      step("Retrieve Payment with shipping cost", shouldContinue, () => {
>>>>>>> Stashed changes
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentConfirmWithShippingCost"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
<<<<<<< Updated upstream

        cy.task(
          "cli_log",
          "Completed step: Retrieve Payment with shipping cost"
        );
=======
>>>>>>> Stashed changes
      });
    });
  });
});
