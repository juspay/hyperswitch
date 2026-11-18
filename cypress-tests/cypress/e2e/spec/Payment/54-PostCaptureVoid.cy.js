import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";

let globalState;

describe("Card - Post-capture void flow test", { retries: 0 }, () => {
  before("seed global state and select connector", function () {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      if (
        shouldIncludeConnector(
          globalState.get("connectorId"),
          CONNECTOR_LISTS.INCLUDE.POST_CAPTURE_VOID
        )
      ) {
        this.skip();
      }
    });
  });

  after("flush global state", () => {
    if (globalState) {
      cy.task("setGlobalState", globalState.data);
    }
  });

  context("Card - post-capture void before capture flow test", () => {
    it("Create Payment Intent -> Reject Post-Capture Void", () => {
      const card = getConnectorDetails(globalState.get("connectorId")).card_pm;

      cy.step("Create Payment Intent", () => {
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          card.PaymentIntent,
          "no_three_ds",
          "manual",
          globalState
        );
      });

      cy.step("Reject Post-Capture Void", () => {
        cy.postCaptureVoidCallTest(card.PostCaptureVoidUncaptured, globalState);
      });
    });
  });

  context("Card - post-capture void after full capture flow test", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Post-Capture Void -> Retrieve Payment -> Reject Duplicate Void", () => {
      const card = getConnectorDetails(globalState.get("connectorId")).card_pm;

      cy.step("Create Payment Intent", () => {
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          card.PaymentIntent,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      cy.step("Payment Methods Call", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment Intent", () => {
        cy.confirmCallTest(
          fixtures.confirmBody,
          card.No3DSAutoCapture,
          true,
          globalState
        );
      });

      cy.step("Post-Capture Void", () => {
        cy.postCaptureVoidCallTest(card.PostCaptureVoidFull, globalState);
      });

      cy.step("Retrieve Payment after Void", () => {
        cy.retrievePostCaptureVoidCallTest(
          card.PostCaptureVoidFull,
          globalState
        );
      });

      cy.step("Reject Duplicate Void", () => {
        cy.postCaptureVoidCallTest(card.PostCaptureVoidDuplicate, globalState);
      });
    });
  });

  context("Card - post-capture void after partial capture flow test", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Partial Capture -> Post-Capture Void -> Retrieve Payment", () => {
      const card = getConnectorDetails(globalState.get("connectorId")).card_pm;

      cy.step("Create Payment Intent", () => {
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          card.PaymentIntent,
          "no_three_ds",
          "manual",
          globalState
        );
      });

      cy.step("Payment Methods Call", () => {
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment Intent", () => {
        cy.confirmCallTest(
          fixtures.confirmBody,
          card.No3DSManualCapture,
          true,
          globalState
        );
      });

      cy.step("Partial Capture", () => {
        cy.captureCallTest(
          fixtures.captureBody,
          card.PartialCapture,
          globalState
        );
      });

      cy.step("Post-Capture Void", () => {
        cy.postCaptureVoidCallTest(card.PostCaptureVoidPartial, globalState);
      });

      cy.step("Retrieve Payment after Void", () => {
        cy.retrievePostCaptureVoidCallTest(
          card.PostCaptureVoidPartial,
          globalState
        );
      });
    });
  });
});
