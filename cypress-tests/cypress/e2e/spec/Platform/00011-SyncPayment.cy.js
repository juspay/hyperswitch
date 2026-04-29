import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Platform - Card Sync Payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Platform acts on behalf of Connected Merchant 1 - Sync Payment",
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

      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation", () => {
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
        });
      });
    }
  );

  context("Connected Merchant 2 makes own payment - Sync Payment", () => {
    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Retrieve Payment after Confirmation", () => {
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

      cy.step("Retrieve Payment after Confirmation", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment after Confirmation"
          );
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data: confirmData });
      });
    });
  });
});
