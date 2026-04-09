import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

const VGS_CONNECTOR_NAME = "vgs";

describe("External Vault (VGS) - Connector Integration Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("1. VGS vault connector setup on existing profile", () => {
    it("Create VGS vault connector (vault_processor) on existing profile", () => {
      const vgsConnectorBody = Cypress._.cloneDeep(
        fixtures.createConnectorBody
      );
      vgsConnectorBody.connector_name = VGS_CONNECTOR_NAME;
      vgsConnectorBody.payment_methods_enabled = [];

      cy.externalVaultConnectorCreateCallTest(vgsConnectorBody, globalState);
    });

    it("Update existing business profile to enable VGS as external vault", () => {
      const vgsMcaId = globalState.get("vaultConnectorId");

      const updateBusinessProfileBody = {
        ...fixtures.businessProfile.bpUpdate,
        is_external_vault_enabled: "enable",
        external_vault_connector_details: {
          vault_connector_id: vgsMcaId,
        },
      };

      cy.UpdateBusinessProfileTest(
        updateBusinessProfileBody,
        false, //is_connector_agnostic_mit_enabled
        false, //collect_billing_details_from_wallet_connector
        false, //collect_shipping_details_from_wallet_connector
        false, //always_collect_billing_details_from_wallet_connector
        false, //always_collect_shipping_details_from_wallet_connector
        globalState
      );
    });
  });

  context(
    "Save card for NoThreeDS automatic capture payment - Create+Confirm [on_session]",
    () => {
      it("Create Customer -> Create and Confirm Payment -> Retrieve Payment -> List Customer PMs -> Create Payment Intent -> Save Card Confirm", () => {
        let shouldContinue = true;

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Create and Confirm Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create and Confirm Payment");
            return;
          }
          const createConfirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardUseNo3DSAutoCapture"];
          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            createConfirmData,
            "no_three_ds",
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(createConfirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Confirm", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment after Confirm");
            return;
          }
          const createConfirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardUseNo3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data: createConfirmData });
          if (!utils.should_continue_further(createConfirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("List Customer Payment Methods", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List Customer Payment Methods");
            return;
          }
          cy.listCustomerPMCallTest(globalState);
        });

        cy.step("Create Payment Intent", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create Payment Intent");
            return;
          }
          const paymentIntentData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PaymentIntent"];
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            paymentIntentData,
            "no_three_ds",
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(paymentIntentData)) {
            shouldContinue = false;
          }
        });

        cy.step("Save Card Confirm Call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Save Card Confirm Call");
            return;
          }
          const saveCardBody = Cypress._.cloneDeep(
            fixtures.saveCardConfirmBody
          );
          const saveCardConfirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardUseNo3DSAutoCapture"];
          cy.saveCardConfirmCallTest(
            saveCardBody,
            saveCardConfirmData,
            globalState
          );
        });
      });
    }
  );

  context(
    "Save card for NoThreeDS manual capture payment - Create+Confirm [on_session]",
    () => {
      it("Create Customer -> Create and Confirm Payment -> Retrieve Payment -> List Customer PMs -> Create Payment Intent -> Save Card Confirm -> Retrieve Payment -> Capture -> Retrieve after Capture", () => {
        let shouldContinue = true;

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Create and Confirm Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create and Confirm Payment");
            return;
          }
          const createConfirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardUseNo3DSAutoCapture"];
          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            createConfirmData,
            "no_three_ds",
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(createConfirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Confirm", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment after Confirm");
            return;
          }
          const createConfirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardUseNo3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data: createConfirmData });
          if (!utils.should_continue_further(createConfirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("List Customer Payment Methods", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List Customer Payment Methods");
            return;
          }
          cy.listCustomerPMCallTest(globalState);
        });

        cy.step("Create Payment Intent (manual capture)", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create Payment Intent");
            return;
          }
          const paymentIntentData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PaymentIntent"];
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            paymentIntentData,
            "no_three_ds",
            "manual",
            globalState
          );
          if (!utils.should_continue_further(paymentIntentData)) {
            shouldContinue = false;
          }
        });

        cy.step("Save Card Confirm Call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Save Card Confirm Call");
            return;
          }
          const saveCardBody = Cypress._.cloneDeep(
            fixtures.saveCardConfirmBody
          );
          const saveCardConfirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardUseNo3DSManualCapture"];
          cy.saveCardConfirmCallTest(
            saveCardBody,
            saveCardConfirmData,
            globalState
          );
          if (!utils.should_continue_further(saveCardConfirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Save Card Confirm", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Save Card Confirm"
            );
            return;
          }
          const saveCardConfirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardUseNo3DSManualCapture"];
          cy.retrievePaymentCallTest({
            globalState,
            data: saveCardConfirmData,
          });
          if (!utils.should_continue_further(saveCardConfirmData)) {
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

  context(
    "Save card for NoThreeDS automatic capture payment [off_session]",
    () => {
      it("Create Customer -> Create and Confirm Payment -> Retrieve Payment -> List Customer PMs -> Create Payment Intent -> Save Card Confirm", () => {
        let shouldContinue = true;

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Create and Confirm Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create and Confirm Payment");
            return;
          }
          const createConfirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardUseNo3DSAutoCaptureOffSession"];
          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            createConfirmData,
            "no_three_ds",
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(createConfirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Confirm", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment after Confirm");
            return;
          }
          const createConfirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardUseNo3DSAutoCaptureOffSession"];
          cy.retrievePaymentCallTest({ globalState, data: createConfirmData });
          if (!utils.should_continue_further(createConfirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("List Customer Payment Methods", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: List Customer Payment Methods");
            return;
          }
          cy.listCustomerPMCallTest(globalState);
        });

        cy.step("Create Payment Intent", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Create Payment Intent");
            return;
          }
          const paymentIntentData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PaymentIntentOffSession"];
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            paymentIntentData,
            "no_three_ds",
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(paymentIntentData)) {
            shouldContinue = false;
          }
        });

        cy.step("Save Card Confirm Call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Save Card Confirm Call");
            return;
          }
          const saveCardBody = Cypress._.cloneDeep(
            fixtures.saveCardConfirmBody
          );
          const saveCardConfirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardConfirmAutoCaptureOffSession"];
          cy.saveCardConfirmCallTest(
            saveCardBody,
            saveCardConfirmData,
            globalState
          );
        });
      });
    }
  );

  context("Remove VGS external vault from business profile", () => {
    it("Remove external_vault_connector_id from business profile", () => {
      const updateBusinessProfileBody = {
        is_external_vault_enabled: "disable",
      };

      cy.UpdateBusinessProfileTest(
        updateBusinessProfileBody,
        false, //is_connector_agnostic_mit_enabled
        false, //collect_billing_details_from_wallet_connector
        false, //collect_shipping_details_from_wallet_connector
        false, //always_collect_billing_details_from_wallet_connector
        false, //always_collect_shipping_details_from_wallet_connector
        globalState
      );
    });
  });
});
