import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

const connectorId = Cypress.env("CONNECTOR");
const describeIfSupported = CONNECTOR_LISTS.INCLUDE.IFRAME_REDIRECTION.includes(
  connectorId
)
  ? describe
  : describe.skip;

describeIfSupported("Iframe Redirection Payment Flow Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Happy Path - Create Payment with Iframe Redirection Enabled", () => {
    it("Create Payment Intent with is_iframe_redirection_enabled -> Confirm Payment -> Verify Redirect Response -> Retrieve Payment", function () {
      let shouldContinue = true;

      cy.step(
        "Create Payment Intent with is_iframe_redirection_enabled",
        () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["IframeRedirectionCreate"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        }
      );

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment with is_iframe_redirection_enabled", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["IframeRedirection"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Verify Redirect Response Contains Iframe URL", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Verify Redirect Response");
          return;
        }

        cy.verifyIframeRedirection(globalState, {
          expectRedirectInsidePopup: true,
        });
      });

      cy.step("Poll Payment Status to Terminal State", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Poll Payment Status");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Edge Case - Iframe Redirection Not Explicitly Enabled", () => {
    it("Create Payment Intent without iframe flag -> Confirm Payment -> Verify redirect still inside popup", function () {
      let shouldContinue = true;

      cy.step(
        "Create Payment Intent without is_iframe_redirection_enabled",
        () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        }
      );

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment without iframe redirection flag", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const fullData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];
        const data = JSON.parse(JSON.stringify(fullData));
        if (data?.Response?.body?.payment_method_data) {
          delete data.Response.body.payment_method_data;
        }

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Verify Redirect Still Inside Popup", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Verify Redirect");
          return;
        }

        cy.verifyIframeRedirection(globalState, {
          expectRedirectInsidePopup: true,
        });
      });
    });
  });

  context("Negative Case - Iframe Redirection with No3DS Auth Type", () => {
    it("Create Payment with iframe flag but no_three_ds -> Confirm -> Verify redirect still inside popup", function () {
      let shouldContinue = true;

      cy.step("Create Payment Intent with iframe flag and no_three_ds", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["IframeRedirectionCreate"];

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

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step(
        "Confirm Payment with no_three_ds on iframe-enabled intent",
        () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }

          const fullData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          const data = JSON.parse(JSON.stringify(fullData));
          if (data?.Response?.body?.payment_method_data) {
            delete data.Response.body.payment_method_data;
          }

          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        }
      );

      cy.step("Verify Iframe Redirect Occurred Without 3DS", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Verify Redirect");
          return;
        }

        cy.verifyIframeRedirection(globalState, {
          expectRedirectInsidePopup: true,
          expectedStatus: "succeeded",
        });
      });
    });
  });
});
