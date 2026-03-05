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

        step("Create Payment Intent", shouldContinue, () => {
          const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];

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

        step("Confirm Payment Intent", shouldContinue, () => {
          const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];

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

        step("Retrieve Payment after Confirmation", shouldContinue, () => {
          const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data: confirmData });

          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        step("Capture Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];

          cy.captureCallTest(fixtures.captureBody, captureData, globalState);

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve Payment after Capture", shouldContinue, () => {
          const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];

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

        step("Create and Confirm Payment", shouldContinue, () => {
          const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];

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

        step("Handle Redirection", shouldContinue, () => {
          const expected_redirection = fixtures.createConfirmPaymentBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        step("Retrieve Payment", shouldContinue, () => {
          const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        step("Capture Payment", shouldContinue, () => {
          const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];

          cy.captureCallTest(fixtures.captureBody, captureData, globalState);

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve Payment after Capture", shouldContinue, () => {
          const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];

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

        step("Create Payment Intent", shouldContinue, () => {
          const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];

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

        step("Confirm Payment Intent", shouldContinue, () => {
          const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];

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

        step("Retrieve Payment after Confirmation", shouldContinue, () => {
          const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data: confirmData });

          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        step("Partial Capture Payment", shouldContinue, () => {
          const partialCaptureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];

          cy.captureCallTest(
            fixtures.captureBody,
            partialCaptureData,
            globalState
          );

          if (!utils.should_continue_further(partialCaptureData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve Payment after Partial Capture", shouldContinue, () => {
          const partialCaptureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];

          cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
        });
      });
    }
  );

  context(
    "Card - ThreeDS Manual Partial Capture payment flow test - Create+Confirm",
    () => {
      it("Create and Confirm Payment -> handle redirection -> Retrieve Payment after Confirmation -> Partial Capture Payment -> Retrieve Payment after Partial Capture", () => {
        let shouldContinue = true;

        step("Create and Confirm Payment", shouldContinue, () => {
          const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];

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

        step("handle redirection", shouldContinue, () => {
          const expected_redirection = fixtures.createConfirmPaymentBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        step("Retrieve Payment after Confirmation", shouldContinue, () => {
          const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        step("Partial Capture Payment", shouldContinue, () => {
          const partialCaptureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];

          cy.captureCallTest(
            fixtures.captureBody,
            partialCaptureData,
            globalState
          );

          if (!utils.should_continue_further(partialCaptureData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve Payment after Partial Capture", shouldContinue, () => {
          const partialCaptureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialCapture"];

          cy.retrievePaymentCallTest({ globalState, data: partialCaptureData });
        });
      });
    }
  );
});