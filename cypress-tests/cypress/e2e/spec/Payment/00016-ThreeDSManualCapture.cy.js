import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import step from "../../../utils/customStep";

let globalState;

describe("Card - ThreeDS Manual payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Card - ThreeDS Manual Full Capture payment flow test - Create and Confirm",
    () => {
      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> handle redirection -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture", () => {
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
            "three_ds",
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
        cy.step("Confirm Payment Intent", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment Intent");
            return;
          }

          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["3DSManualCapture"];
=======
        step("Confirm Payment Intent", shouldContinue, () => {
          const confirmData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSManualCapture"];
>>>>>>> Stashed changes

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

        step("handle redirection", shouldContinue, () => {
          const expected_redirection = fixtures.confirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
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

          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data: confirmData });

          cy.task(
            "cli_log",
            "Completed step: Retrieve Payment after Confirmation"
          );
        });

        cy.step("Capture Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Capture Payment");
            return;
          }

          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
=======
        step("Retrieve Payment after Confirmation", shouldContinue, () => {
          const confirmData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data: confirmData });

          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        step("Capture Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];
>>>>>>> Stashed changes

          cy.captureCallTest(fixtures.captureBody, captureData, globalState);

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

<<<<<<< Updated upstream
        cy.step("Retrieve Payment after Capture", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment after Capture");
            return;
          }

          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
=======
        step("Retrieve Payment after Capture", shouldContinue, () => {
          const captureData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];
>>>>>>> Stashed changes

          cy.retrievePaymentCallTest({ globalState, data: captureData });
        });
      });
    }
  );

  context(
    "Card - ThreeDS Manual Full Capture payment flow test - Create+Confirm",
    () => {
      it("Create and Confirm Payment -> Handle Redirection -> Retrieve Payment -> Capture Payment -> Retrieve Payment after Capture", () => {
        let shouldContinue = true;

<<<<<<< Updated upstream
        cy.step("Create and Confirm Payment", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSManualCapture"];
=======
        step("Create and Confirm Payment", shouldContinue, () => {
          const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
            "3DSManualCapture"
          ];
>>>>>>> Stashed changes

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "three_ds",
            "manual",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

<<<<<<< Updated upstream
        cy.step("Handle Redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle Redirection");
            return;
          }

          const expected_redirection =
            fixtures.createConfirmPaymentBody["return_url"];
=======
        step("Handle Redirection", shouldContinue, () => {
          const expected_redirection = fixtures.createConfirmPaymentBody["return_url"];
>>>>>>> Stashed changes
          cy.handleRedirection(globalState, expected_redirection);
        });

<<<<<<< Updated upstream
        cy.step("Retrieve Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }

          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSManualCapture"];
=======
        step("Retrieve Payment", shouldContinue, () => {
          const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
            "3DSManualCapture"
          ];
>>>>>>> Stashed changes

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

<<<<<<< Updated upstream
        cy.step("Capture Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Capture Payment");
            return;
          }

          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
=======
        step("Capture Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];
>>>>>>> Stashed changes

          cy.captureCallTest(fixtures.captureBody, captureData, globalState);

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

<<<<<<< Updated upstream
        cy.step("Retrieve Payment after Capture", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment after Capture");
            return;
          }

          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];
=======
        step("Retrieve Payment after Capture", shouldContinue, () => {
          const captureData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];
>>>>>>> Stashed changes

          cy.retrievePaymentCallTest({ globalState, data: captureData });
        });
      });
    }
  );

  context(
    "Card - ThreeDS Manual Partial Capture payment flow test - Create and Confirm",
    () => {
      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> handle redirection -> Retrieve Payment after Confirmation -> Partial Capture Payment -> Retrieve Payment after Partial Capture", () => {
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
            "three_ds",
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
        cy.step("Confirm Payment Intent", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment Intent");
            return;
          }

          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["3DSManualCapture"];
=======
        step("Confirm Payment Intent", shouldContinue, () => {
          const confirmData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSManualCapture"];
>>>>>>> Stashed changes

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

        step("handle redirection", shouldContinue, () => {
          const expected_redirection = fixtures.confirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
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

          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data: confirmData });

          cy.task(
            "cli_log",
            "Completed step: Retrieve Payment after Confirmation"
          );
=======
        step("Retrieve Payment after Confirmation", shouldContinue, () => {
          const confirmData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data: confirmData });

          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
>>>>>>> Stashed changes
        });

        step("Partial Capture Payment", shouldContinue, () => {
          const partialCaptureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialCapture"];

          cy.captureCallTest(
            fixtures.captureBody,
            partialCaptureData,
            globalState
          );

          if (!utils.should_continue_further(partialCaptureData)) {
            shouldContinue = false;
          }
        });

<<<<<<< Updated upstream
        cy.step("Retrieve Payment after Partial Capture", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Partial Capture"
            );
            return;
          }

=======
        step("Retrieve Payment after Partial Capture", shouldContinue, () => {
>>>>>>> Stashed changes
          const partialCaptureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialCapture"];

          cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
<<<<<<< Updated upstream

          cy.task(
            "cli_log",
            "Completed step: Retrieve Payment after Partial Capture"
          );
=======
>>>>>>> Stashed changes
        });
      });
    }
  );

  context(
    "Card - ThreeDS Manual Partial Capture payment flow test - Create+Confirm",
    () => {
      it("Create and Confirm Payment -> handle redirection -> Retrieve Payment after Confirmation -> Partial Capture Payment -> Retrieve Payment after Partial Capture", () => {
        let shouldContinue = true;

<<<<<<< Updated upstream
        cy.step("Create and Confirm Payment", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSManualCapture"];
=======
        step("Create and Confirm Payment", shouldContinue, () => {
          const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
            "3DSManualCapture"
          ];
>>>>>>> Stashed changes

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "three_ds",
            "manual",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

<<<<<<< Updated upstream
        cy.step("handle redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: handle redirection");
            return;
          }

          const expected_redirection =
            fixtures.createConfirmPaymentBody["return_url"];
=======
        step("handle redirection", shouldContinue, () => {
          const expected_redirection = fixtures.createConfirmPaymentBody["return_url"];
>>>>>>> Stashed changes
          cy.handleRedirection(globalState, expected_redirection);
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

          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          cy.task(
            "cli_log",
            "Completed step: Retrieve Payment after Confirmation"
          );
=======
        step("Retrieve Payment after Confirmation", shouldContinue, () => {
          const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
            "3DSManualCapture"
          ];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
>>>>>>> Stashed changes
        });

        step("Partial Capture Payment", shouldContinue, () => {
          const partialCaptureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialCapture"];

          cy.captureCallTest(
            fixtures.captureBody,
            partialCaptureData,
            globalState
          );

          if (!utils.should_continue_further(partialCaptureData)) {
            shouldContinue = false;
          }
        });

<<<<<<< Updated upstream
        cy.step("Retrieve Payment after Partial Capture", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Partial Capture"
            );
            return;
          }

=======
        step("Retrieve Payment after Partial Capture", shouldContinue, () => {
>>>>>>> Stashed changes
          const partialCaptureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialCapture"];

          cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
<<<<<<< Updated upstream

          cy.task(
            "cli_log",
            "Completed step: Retrieve Payment after Partial Capture"
          );
=======
>>>>>>> Stashed changes
        });
      });
    }
  );
});
