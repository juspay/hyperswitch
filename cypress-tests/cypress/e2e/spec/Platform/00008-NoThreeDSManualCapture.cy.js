import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Platform - Card NoThreeDS Manual Capture payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Platform acts on behalf of Connected Merchant 1 - NoThreeDS Manual Full Capture",
    () => {
      let savedProfileId, savedConnectorId, savedMerchantConnectorId;

      before(() => {
        savedProfileId = globalState.get("profileId");
        savedConnectorId = globalState.get("connectorId");
        savedMerchantConnectorId = globalState.get("merchantConnectorId");

        globalState.set("profileId", globalState.get("profileId_CM1"));
        globalState.set("connectorId", "stripe");
        globalState.set(
          "merchantConnectorId",
          globalState.get("connectorId_CM1")
        );
      });

      after(() => {
        globalState.set("profileId", savedProfileId);
        globalState.set("connectorId", savedConnectorId);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });

      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent for CM1 using header", () => {
          const data =
            getConnectorDetails("stripe")["card_pm"]["PaymentIntent"];

          cy.createPaymentIntentWithHeaderCallTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "manual",
            globalState,
            globalState.get("connectedMerchantId_1")
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
          globalState.set(
            "publishableKey",
            globalState.get("publishableKey_CM1")
          );

          cy.paymentMethodsCallTest(globalState).then(() => {
            globalState.set("publishableKey", savedPublishableKey);
          });
        });

        cy.step("Confirm Payment Intent for CM1 using header", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment Intent");
            return;
          }
          const data =
            getConnectorDetails("stripe")["card_pm"]["No3DSManualCapture"];

          cy.confirmPaymentWithHeaderCallTest(
            fixtures.confirmBody,
            data,
            true,
            globalState,
            globalState.get("connectedMerchantId_1")
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Confirmation", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Confirmation"
            );
            return;
          }
          const data =
            getConnectorDetails("stripe")["card_pm"]["No3DSManualCapture"];

          cy.retrievePaymentWithHeaderCallTest({
            globalState,
            connectedMerchantId: globalState.get("connectedMerchantId_1"),
            data,
          });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Capture Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Capture Payment");
            return;
          }
          const captureData =
            getConnectorDetails("stripe")["card_pm"]["Capture"];

          cy.captureCallTest(fixtures.captureBody, captureData, globalState);

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Capture", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment after Capture");
            return;
          }
          const captureData =
            getConnectorDetails("stripe")["card_pm"]["Capture"];

          cy.retrievePaymentWithHeaderCallTest({
            globalState,
            connectedMerchantId: globalState.get("connectedMerchantId_1"),
            data: captureData,
          });
        });
      });
    }
  );

  context(
    "Connected Merchant 1 makes own payment - NoThreeDS Manual Full Capture",
    () => {
      let savedApiKey,
        savedProfileId,
        savedPublishableKey,
        savedConnectorId,
        savedMerchantConnectorId;

      before(() => {
        savedApiKey = globalState.get("apiKey");
        savedProfileId = globalState.get("profileId");
        savedPublishableKey = globalState.get("publishableKey");
        savedConnectorId = globalState.get("connectorId");
        savedMerchantConnectorId = globalState.get("merchantConnectorId");

        globalState.set("apiKey", globalState.get("apiKey_CM1"));
        globalState.set("profileId", globalState.get("profileId_CM1"));
        globalState.set("publishableKey", globalState.get("publishableKey_CM1"));
        globalState.set("connectorId", "stripe");
        globalState.set(
          "merchantConnectorId",
          globalState.get("connectorId_CM1")
        );
      });

      after(() => {
        globalState.set("apiKey", savedApiKey);
        globalState.set("profileId", savedProfileId);
        globalState.set("publishableKey", savedPublishableKey);
        globalState.set("connectorId", savedConnectorId);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });

      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

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

        cy.step("Payment Methods Call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Payment Methods Call");
            return;
          }
          cy.paymentMethodsCallTest(globalState);
        });

        cy.step("Confirm Payment Intent", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment Intent");
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSManualCapture"];

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
          )["card_pm"]["No3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data: confirmData });

          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Capture Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Capture Payment");
            return;
          }
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];

          cy.captureCallTest(fixtures.captureBody, captureData, globalState);

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Capture", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment after Capture");
            return;
          }
          const captureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data: captureData });
        });
      });
    }
  );
});
