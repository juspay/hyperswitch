import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Platform - Card ThreeDS payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Platform acts on behalf of Connected Merchant 1 - ThreeDS", () => {
    let savedApiKey,
      savedPublishableKey,
      savedProfileId,
      savedMerchantConnectorId;

    before(() => {
      savedApiKey = globalState.get("apiKey");
      savedPublishableKey = globalState.get("publishableKey");
      savedProfileId = globalState.get("profileId");
      savedMerchantConnectorId = globalState.get("merchantConnectorId");

      globalState.set("apiKey", globalState.get("platformApiKey"));
      globalState.set(
        "publishableKey",
        globalState.get("platformPublishableKey")
      );
      globalState.set("profileId", globalState.get("profileIdCm1"));
      globalState.set("merchantConnectorId", globalState.get("connectorIdCm1"));
    });

    after(() => {
      globalState.set("apiKey", savedApiKey);
      globalState.set("publishableKey", savedPublishableKey);
      globalState.set("profileId", savedProfileId);
      globalState.set("merchantConnectorId", savedMerchantConnectorId);
    });

    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment -> Handle Redirection", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent for CM1 using header", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "three_ds",
          "automatic",
          globalState,
          globalState.get("connectedMerchantId1")
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
        const savedPublishableKey = globalState.get("publishableKey");
        globalState.set("publishableKey", globalState.get("publishableKeyCm1"));
        cy.paymentMethodsCallTest(globalState).then(() => {
          globalState.set("publishableKey", savedPublishableKey);
        });
      });

      cy.step("Confirm Payment for CM1 using header", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];

        cy.confirmCallTest(
          fixtures.confirmBody,
          data,
          true,
          globalState,
          globalState.get("connectedMerchantId1")
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Handle Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });
    });
  });

  context("Connected Merchant 2 makes own payment - ThreeDS", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment -> Handle Redirection", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
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
      });

      cy.step("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSAutoCapture"];

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

      cy.step("Handle Redirection", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle Redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });
    });
  });
});
