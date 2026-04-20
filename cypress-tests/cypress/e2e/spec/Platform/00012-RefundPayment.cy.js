import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Platform - Card Refund Payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Platform acts on behalf of Connected Merchant 1 - Full Refund for Auto-Capture payment",
    () => {
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
        globalState.set(
          "merchantConnectorId",
          globalState.get("connectorIdCm1")
        );
      });

      after(() => {
        globalState.set("apiKey", savedApiKey);
        globalState.set("publishableKey", savedPublishableKey);
        globalState.set("profileId", savedProfileId);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });

      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Refund Payment -> Sync Refund Payment", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent for CM1 using header", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
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
          globalState.set(
            "publishableKey",
            globalState.get("publishableKeyCm1")
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
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

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
          ]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({
            globalState,
            connectedMerchantId: globalState.get("connectedMerchantId1"),
            data,
          });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Refund Payment");
            return;
          }
          const refundData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Refund"];

          cy.refundCallTest(
            fixtures.refundBody,
            refundData,
            globalState,
            globalState.get("connectedMerchantId1")
          );

          if (!utils.should_continue_further(refundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Sync Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Sync Refund Payment");
            return;
          }
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];

          cy.syncRefundCallTest(
            syncRefundData,
            globalState,
            globalState.get("connectedMerchantId1")
          );
        });
      });
    }
  );

  context(
    "Connected Merchant 2 makes own payment - Full Refund for Auto-Capture payment",
    () => {
      let savedApiKey,
        savedPublishableKey,
        savedMerchantConnectorId;

      before(() => {
        savedApiKey = globalState.get("apiKey");
        savedPublishableKey = globalState.get("publishableKey");
        savedMerchantConnectorId = globalState.get("merchantConnectorId");

        globalState.set("apiKey", globalState.get("apiKeyCm2"));
        globalState.set("publishableKey", globalState.get("publishableKeyCm2"));
        globalState.set("merchantConnectorId", globalState.get("connectorIdCm2"));
      });

      after(() => {
        globalState.set("apiKey", savedApiKey);
        globalState.set("publishableKey", savedPublishableKey);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });

      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Refund Payment -> Sync Refund Payment", () => {
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
          )["card_pm"]["No3DSAutoCapture"];

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
          )["card_pm"]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data: confirmData });

          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Refund Payment");
            return;
          }
          const refundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["Refund"];

          cy.refundCallTest(fixtures.refundBody, refundData, globalState);

          if (!utils.should_continue_further(refundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Sync Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Sync Refund Payment");
            return;
          }
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];

          cy.syncRefundCallTest(syncRefundData, globalState);
        });
      });
    }
  );

  context(
    "Platform acts on behalf of Connected Merchant 1 - Partial Refund for Auto-Capture payment",
    () => {
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
        globalState.set(
          "merchantConnectorId",
          globalState.get("connectorIdCm1")
        );
      });

      after(() => {
        globalState.set("apiKey", savedApiKey);
        globalState.set("publishableKey", savedPublishableKey);
        globalState.set("profileId", savedProfileId);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });

      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent for CM1 using header", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
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
          globalState.set(
            "publishableKey",
            globalState.get("publishableKeyCm1")
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
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

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
          ]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({
            globalState,
            connectedMerchantId: globalState.get("connectedMerchantId1"),
            data,
          });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Partial Refund Payment");
            return;
          }
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialRefund"];

          cy.refundCallTest(
            fixtures.refundBody,
            partialRefundData,
            globalState,
            globalState.get("connectedMerchantId1")
          );

          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Refund Payment - 2nd Attempt", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Partial Refund Payment - 2nd Attempt"
            );
            return;
          }
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialRefund"];

          cy.refundCallTest(
            fixtures.refundBody,
            partialRefundData,
            globalState,
            globalState.get("connectedMerchantId1")
          );

          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Sync Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Sync Refund Payment");
            return;
          }
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];

          cy.syncRefundCallTest(
            syncRefundData,
            globalState,
            globalState.get("connectedMerchantId1")
          );
        });
      });
    }
  );

  context(
    "Connected Merchant 2 makes own payment - Partial Refund for Auto-Capture payment",
    () => {
      let savedApiKey,
        savedPublishableKey,
        savedMerchantConnectorId;

      before(() => {
        savedApiKey = globalState.get("apiKey");
        savedPublishableKey = globalState.get("publishableKey");
        savedMerchantConnectorId = globalState.get("merchantConnectorId");

        globalState.set("apiKey", globalState.get("apiKeyCm2"));
        globalState.set("publishableKey", globalState.get("publishableKeyCm2"));
        globalState.set("merchantConnectorId", globalState.get("connectorIdCm2"));
      });

      after(() => {
        globalState.set("apiKey", savedApiKey);
        globalState.set("publishableKey", savedPublishableKey);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });

      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment", () => {
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
          )["card_pm"]["No3DSAutoCapture"];

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
          )["card_pm"]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data: confirmData });

          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Partial Refund Payment");
            return;
          }
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialRefund"];

          cy.refundCallTest(
            fixtures.refundBody,
            partialRefundData,
            globalState
          );

          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Refund Payment - 2nd Attempt", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Partial Refund Payment - 2nd Attempt"
            );
            return;
          }
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialRefund"];

          cy.refundCallTest(
            fixtures.refundBody,
            partialRefundData,
            globalState
          );

          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Sync Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Sync Refund Payment");
            return;
          }
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];

          cy.syncRefundCallTest(syncRefundData, globalState);
        });
      });
    }
  );

  context(
    "Platform acts on behalf of Connected Merchant 1 - Full Refund for Manual-Capture payment",
    () => {
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
        globalState.set(
          "merchantConnectorId",
          globalState.get("connectorIdCm1")
        );
      });

      after(() => {
        globalState.set("apiKey", savedApiKey);
        globalState.set("publishableKey", savedPublishableKey);
        globalState.set("profileId", savedProfileId);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });

      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture -> Refund Payment -> Sync Refund Payment", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent for CM1 using header", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "manual",
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
          globalState.set(
            "publishableKey",
            globalState.get("publishableKeyCm1")
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
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

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
          ]["No3DSManualCapture"];

          cy.retrievePaymentCallTest({
            globalState,
            connectedMerchantId: globalState.get("connectedMerchantId1"),
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
          const captureData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.captureCallTest(
            fixtures.captureBody,
            captureData,
            globalState,
            globalState.get("connectedMerchantId1")
          );

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Capture", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment after Capture");
            return;
          }
          const captureData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.retrievePaymentCallTest({
            globalState,
            connectedMerchantId: globalState.get("connectedMerchantId1"),
            data: captureData,
          });

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Refund Payment");
            return;
          }
          const refundData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["manualPaymentRefund"];
          const newRefundData = {
            ...refundData,
            Response: refundData.ResponseCustom || refundData.Response,
          };

          cy.refundCallTest(
            fixtures.refundBody,
            newRefundData,
            globalState,
            globalState.get("connectedMerchantId1")
          );

          if (!utils.should_continue_further(refundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Sync Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Sync Refund Payment");
            return;
          }
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];

          cy.syncRefundCallTest(
            syncRefundData,
            globalState,
            globalState.get("connectedMerchantId1")
          );
        });
      });
    }
  );

  context(
    "Connected Merchant 2 makes own payment - Full Refund for Manual-Capture payment",
    () => {
      let savedApiKey,
        savedPublishableKey,
        savedMerchantConnectorId;

      before(() => {
        savedApiKey = globalState.get("apiKey");
        savedPublishableKey = globalState.get("publishableKey");
        savedMerchantConnectorId = globalState.get("merchantConnectorId");

        globalState.set("apiKey", globalState.get("apiKeyCm2"));
        globalState.set("publishableKey", globalState.get("publishableKeyCm2"));
        globalState.set("merchantConnectorId", globalState.get("connectorIdCm2"));
      });

      after(() => {
        globalState.set("apiKey", savedApiKey);
        globalState.set("publishableKey", savedPublishableKey);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });

      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture -> Refund Payment -> Sync Refund Payment", () => {
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

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Refund Payment");
            return;
          }
          const refundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["manualPaymentRefund"];
          const newRefundData = {
            ...refundData,
            Response: refundData.ResponseCustom || refundData.Response,
          };

          cy.refundCallTest(fixtures.refundBody, newRefundData, globalState);

          if (!utils.should_continue_further(refundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Sync Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Sync Refund Payment");
            return;
          }
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];

          cy.syncRefundCallTest(syncRefundData, globalState);
        });
      });
    }
  );

  context(
    "Platform acts on behalf of Connected Merchant 1 - Partial Refund for Manual-Capture payment",
    () => {
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
        globalState.set(
          "merchantConnectorId",
          globalState.get("connectorIdCm1")
        );
      });

      after(() => {
        globalState.set("apiKey", savedApiKey);
        globalState.set("publishableKey", savedPublishableKey);
        globalState.set("profileId", savedProfileId);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });

      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment -> List Refunds", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent for CM1 using header", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "manual",
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
          globalState.set(
            "publishableKey",
            globalState.get("publishableKeyCm1")
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
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

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
          ]["No3DSManualCapture"];

          cy.retrievePaymentCallTest({
            globalState,
            connectedMerchantId: globalState.get("connectedMerchantId1"),
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
          const captureData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.captureCallTest(
            fixtures.captureBody,
            captureData,
            globalState,
            globalState.get("connectedMerchantId1")
          );

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Capture", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment after Capture");
            return;
          }
          const captureData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.retrievePaymentCallTest({
            globalState,
            connectedMerchantId: globalState.get("connectedMerchantId1"),
            data: captureData,
          });

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Partial Refund Payment");
            return;
          }
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["manualPaymentPartialRefund"];
          const newPartialRefundData = {
            ...partialRefundData,
            Response:
              partialRefundData.ResponseCustom || partialRefundData.Response,
          };

          cy.refundCallTest(
            fixtures.refundBody,
            newPartialRefundData,
            globalState,
            globalState.get("connectedMerchantId1")
          );

          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Refund Payment - 2nd Attempt", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Partial Refund Payment - 2nd Attempt"
            );
            return;
          }
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["manualPaymentPartialRefund"];
          const newPartialRefundData = {
            ...partialRefundData,
            Response:
              partialRefundData.ResponseCustom || partialRefundData.Response,
          };

          cy.refundCallTest(
            fixtures.refundBody,
            newPartialRefundData,
            globalState,
            globalState.get("connectedMerchantId1")
          );

          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Sync Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Sync Refund Payment");
            return;
          }
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];

          cy.syncRefundCallTest(
            syncRefundData,
            globalState,
            globalState.get("connectedMerchantId1")
          );

          if (!utils.should_continue_further(syncRefundData)) {
            shouldContinue = false;
          }
        });

        cy.step("List Refunds", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List Refunds");
            return;
          }
          cy.listRefundCallTest(
            fixtures.listRefundCall,
            globalState,
            globalState.get("connectedMerchantId1")
          );
        });
      });
    }
  );

  context(
    "Connected Merchant 2 makes own payment - Partial Refund for Manual-Capture payment",
    () => {
      let savedApiKey,
        savedPublishableKey,
        savedMerchantConnectorId;

      before(() => {
        savedApiKey = globalState.get("apiKey");
        savedPublishableKey = globalState.get("publishableKey");
        savedMerchantConnectorId = globalState.get("merchantConnectorId");

        globalState.set("apiKey", globalState.get("apiKeyCm2"));
        globalState.set("publishableKey", globalState.get("publishableKeyCm2"));
        globalState.set("merchantConnectorId", globalState.get("connectorIdCm2"));
      });

      after(() => {
        globalState.set("apiKey", savedApiKey);
        globalState.set("publishableKey", savedPublishableKey);
        globalState.set("merchantConnectorId", savedMerchantConnectorId);
      });

      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation -> Capture Payment -> Retrieve Payment after Capture -> Partial Refund Payment -> Partial Refund Payment - 2nd Attempt -> Sync Refund Payment -> List Refunds", () => {
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

          if (!utils.should_continue_further(captureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Partial Refund Payment");
            return;
          }
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["manualPaymentPartialRefund"];
          const newPartialRefundData = {
            ...partialRefundData,
            Response:
              partialRefundData.ResponseCustom || partialRefundData.Response,
          };

          cy.refundCallTest(
            fixtures.refundBody,
            newPartialRefundData,
            globalState
          );

          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Partial Refund Payment - 2nd Attempt", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Partial Refund Payment - 2nd Attempt"
            );
            return;
          }
          const partialRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["manualPaymentPartialRefund"];
          const newPartialRefundData = {
            ...partialRefundData,
            Response:
              partialRefundData.ResponseCustom || partialRefundData.Response,
          };

          cy.refundCallTest(
            fixtures.refundBody,
            newPartialRefundData,
            globalState
          );

          if (!utils.should_continue_further(partialRefundData)) {
            shouldContinue = false;
          }
        });

        cy.step("Sync Refund Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Sync Refund Payment");
            return;
          }
          const syncRefundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SyncRefund"];

          cy.syncRefundCallTest(syncRefundData, globalState);

          if (!utils.should_continue_further(syncRefundData)) {
            shouldContinue = false;
          }
        });

        cy.step("List Refunds", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List Refunds");
            return;
          }
          cy.listRefundCallTest(fixtures.listRefundCall, globalState);
        });
      });
    }
  );
});
