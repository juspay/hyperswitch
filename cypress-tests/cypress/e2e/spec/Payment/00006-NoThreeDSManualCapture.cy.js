import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

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

  context("Card - NoThreeDS Manual Full Capture payment flow test - Create and Confirm", () => {
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Retrieve Payment after Confirmation + Capture Payment + Retrieve Payment after Capture", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const captureData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.step("Capture Payment", () =>
        cy.captureCallTest(fixtures.captureBody, captureData, globalState)
      );

      if (!utils.should_continue_further(captureData)) return;

      cy.step("Retrieve Payment after Capture", () =>
        cy.retrievePaymentCallTest({ globalState, data: captureData })
      );
    });
  });

  context("Card - NoThreeDS Manual Full Capture payment flow test - Create+Confirm", () => {
    it("Create and Confirm Payment Intent + Retrieve Payment after Confirmation + Capture Payment + Retrieve Payment after Capture", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSManualCapture"
      ];

      cy.step("Create and Confirm Payment Intent", () =>
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data })
      );

      const captureData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.step("Capture Payment", () =>
        cy.captureCallTest(fixtures.captureBody, captureData, globalState)
      );

      if (!utils.should_continue_further(captureData)) return;

      cy.step("Retrieve Payment after Capture", () =>
        cy.retrievePaymentCallTest({ globalState, data: captureData })
      );
    });
  });

  context("Card - NoThreeDS Manual Partial Capture payment flow test - Create and Confirm", () => {
    it("Create Payment Intent + Payment Methods Call + Confirm Payment Intent + Retrieve Payment after Confirmation + Partial Capture Payment + Retrieve Payment after Partial Capture", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.step("Create Payment Intent", () =>
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Payment Methods Call", () =>
        cy.paymentMethodsCallTest(globalState)
      );

      const confirmData = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSManualCapture"];

      cy.step("Confirm Payment Intent", () =>
        cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
      );

      if (!utils.should_continue_further(confirmData)) return;

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data: confirmData })
      );

      const partialCaptureData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["PartialCapture"];

      cy.step("Partial Capture Payment", () =>
        cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState)
      );

      if (!utils.should_continue_further(partialCaptureData)) return;

      cy.step("Retrieve Payment after Partial Capture", () =>
        cy.retrievePaymentCallTest({ globalState, data: partialCaptureData })
      );
    });
  });

  context("Card - NoThreeDS Manual Partial Capture payment flow test - Create+Confirm", () => {
    it("Create and Confirm Payment Intent + Retrieve Payment after Confirmation + Partial Capture Payment + Retrieve Payment after Partial Capture", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSManualCapture"
      ];
      cy.step("Create and Confirm Payment Intent", () =>
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        )
      );

      if (!utils.should_continue_further(data)) return;

      cy.step("Retrieve Payment after Confirmation", () =>
        cy.retrievePaymentCallTest({ globalState, data })
      );

      const partialCaptureData = getConnectorDetails(
        globalState.get("connectorId")
      )["card_pm"]["PartialCapture"];

      cy.step("Partial Capture Payment", () =>
        cy.captureCallTest(fixtures.captureBody, partialCaptureData, globalState)
      );

      if (!utils.should_continue_further(partialCaptureData)) return;

      cy.step("Retrieve Payment after Partial Capture", () =>
        cy.retrievePaymentCallTest({ globalState, data: partialCaptureData })
      );
    });
  });
});
