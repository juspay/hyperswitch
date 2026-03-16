import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Platform - Card NoThreeDS payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Platform acts on behalf of Connected Merchant 1 - NoThreeDS",
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

      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment -> Retrieve Payment", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent for CM1 using header", () => {
          const data =
            getConnectorDetails("stripe")["card_pm"]["PaymentIntent"];

          cy.createPaymentIntentWithHeaderCallTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "automatic",
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

        cy.step("Confirm Payment for CM1 using header", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const data =
            getConnectorDetails("stripe")["card_pm"]["No3DSAutoCapture"];

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

        cy.step("Retrieve Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data =
            getConnectorDetails("stripe")["card_pm"]["No3DSAutoCapture"];

          cy.retrievePaymentWithHeaderCallTest({
            globalState,
            connectedMerchantId: globalState.get("connectedMerchantId_1"),
            data,
          });
        });
      });
    }
  );

  context("Connected Merchant 1 makes own payment - NoThreeDS", () => {
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

    it("Create Payment Intent -> Payment Methods Call -> Confirm Payment -> Retrieve Payment", () => {
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

      cy.step("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
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

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
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
