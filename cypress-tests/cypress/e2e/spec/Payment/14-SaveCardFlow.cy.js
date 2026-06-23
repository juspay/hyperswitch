import * as fixtures from "../../../fixtures/imports";
import { generateRandomName } from "../../../utils/RequestBodyUtils";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - SaveCard payment flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Save card for NoThreeDS automatic capture payment- Create+Confirm [on_session]",
    () => {
      it("Create Customer -> Create and Confirm Payment -> Retrieve Payment after Confirm -> List Customer Payment Methods -> Create Payment Intent -> Save Card Confirm Call", () => {
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
    "Save card for NoThreeDS manual full capture payment- Create+Confirm [on_session]",
    () => {
      it("Create Customer -> Create and Confirm Payment -> Retrieve Payment after Confirm -> List Customer Payment Methods -> Create Payment Intent -> Save Card Confirm Call -> Retrieve Payment after Save Card Confirm -> Capture Payment -> Retrieve Payment after Capture", () => {
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
    "Save card for NoThreeDS manual partial capture payment- Create + Confirm [on_session]",
    () => {
      it("Create Customer -> Create and Confirm Payment -> Retrieve Payment after Confirm -> List Customer Payment Methods -> Create Payment Intent -> Save Card Confirm Call -> Retrieve Payment after Save Card Confirm -> Partial Capture Payment -> Retrieve Payment after Partial Capture", () => {
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

        cy.step("Partial Capture Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Partial Capture Payment");
            return;
          }
          const partialCaptureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialCapture"];
          cy.captureCallTest(
            fixtures.captureBody,
            partialCaptureData,
            globalState
          );
          if (!utils.should_continue_further(partialCaptureData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after Partial Capture", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Partial Capture"
            );
            return;
          }
          const partialCaptureData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PartialCapture"];
          cy.retrievePaymentCallTest({
            globalState,
            data: partialCaptureData,
          });
        });
      });
    }
  );

  context(
    "Save card for NoThreeDS automatic capture payment [off_session]",
    () => {
      it("Create Customer -> Create and Confirm Payment -> Retrieve Payment after Confirm -> List Customer Payment Methods -> Create Payment Intent -> Save Card Confirm Call", () => {
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

  context(
    "Save card for NoThreeDS manual capture payment- Create+Confirm [off_session]",
    () => {
      it("Create Customer -> Create and Confirm Payment -> Retrieve Payment after Confirm -> Capture Payment -> Retrieve Payment after Capture -> List Customer Payment Methods -> Create Payment Intent -> Save Card Confirm Call -> Retrieve Payment after Save Card Confirm -> Capture Payment -> Retrieve Payment after Capture", () => {
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
          )["card_pm"]["SaveCardUseNo3DSManualCaptureOffSession"];
          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            createConfirmData,
            "no_three_ds",
            "manual",
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
          )["card_pm"]["SaveCardUseNo3DSManualCaptureOffSession"];
          cy.retrievePaymentCallTest({ globalState, data: createConfirmData });
          if (!utils.should_continue_further(createConfirmData)) {
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
          )["card_pm"]["SaveCardConfirmManualCaptureOffSession"];
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
          )["card_pm"]["SaveCardConfirmManualCaptureOffSession"];
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
    "Save card for NoThreeDS automatic capture payment - create and confirm [off_session]",
    () => {
      it("Create Customer -> Create Payment Intent -> Confirm Payment -> Retrieve Payment after Confirm -> List Customer Payment Methods -> Create Payment Intent for Subsequent Payment -> Save Card Confirm Call for Subsequent Payment", () => {
        let shouldContinue = true;

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
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

        cy.step("Confirm Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardUseNo3DSAutoCaptureOffSession"];
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

        cy.step("Retrieve Payment after Confirm", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment after Confirm");
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardUseNo3DSAutoCaptureOffSession"];
          cy.retrievePaymentCallTest({ globalState, data: confirmData });
          if (!utils.should_continue_further(confirmData)) {
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

        cy.step("Create Payment Intent for Subsequent Payment", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Create Payment Intent for Subsequent Payment"
            );
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

        cy.step("Save Card Confirm Call for Subsequent Payment", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Save Card Confirm Call for Subsequent Payment"
            );
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

  context(
    "Use billing address from payment method during subsequent payment [off_session]",
    () => {
      it("Create Customer -> Create Payment Intent -> Confirm Payment -> list Customer Payment Methods -> Create Payment Intent for Subsequent Payment -> Save Card Confirm Call for Subsequent Payment without Billing Address", () => {
        let shouldContinue = true;

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
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

        cy.step("Confirm Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment");
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["SaveCardUseNo3DSAutoCaptureOffSession"];
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

        cy.step("list Customer Payment Methods", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: list Customer Payment Methods");
            return;
          }
          cy.listCustomerPMCallTest(globalState);
        });

        cy.step("Create Payment Intent for Subsequent Payment", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Create Payment Intent for Subsequent Payment"
            );
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

        cy.step(
          "Save Card Confirm Call for Subsequent Payment without Billing Address",
          () => {
            if (!shouldContinue) {
              cy.task(
                "cli_log",
                "Skipping step: Save Card Confirm Call for Subsequent Payment without Billing Address"
              );
              return;
            }
            const saveCardBody = Cypress._.cloneDeep(
              fixtures.saveCardConfirmBody
            );
            const saveCardConfirmData = getConnectorDetails(
              globalState.get("connectorId")
            )["card_pm"]["SaveCardConfirmAutoCaptureOffSessionWithoutBilling"];
            cy.saveCardConfirmCallTest(
              saveCardBody,
              saveCardConfirmData,
              globalState
            );
          }
        );
      });
    }
  );

  context(
    "Check if card fields are populated when saving card again after a metadata update",
    () => {
      it("Create Customer -> Create and Confirm Payment -> Retrieve Payment after Confirm -> Create and Confirm Payment again with updated card holder name in metadata -> Retrieve Customer Payment Methods", () => {
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
          cy.listCustomerPMCallTest(globalState);
        });

        cy.step(
          "Create and Confirm Payment again with updated card holder name in metadata",
          () => {
            if (!shouldContinue) {
              cy.task(
                "cli_log",
                "Skipping step: Create and Confirm Payment again with updated card holder name in metadata"
              );
              return;
            }
            const createConfirmData = getConnectorDetails(
              globalState.get("connectorId")
            )["card_pm"]["SaveCardUseNo3DSAutoCapture"];
            const card_holder_name = generateRandomName();
            const newData = {
              ...createConfirmData,
              Request: {
                ...createConfirmData.Request,
                payment_method_data: {
                  card: {
                    ...createConfirmData.Request.payment_method_data.card,
                    card_holder_name: card_holder_name,
                  },
                },
              },
            };
            cy.createConfirmPaymentTest(
              fixtures.createConfirmPaymentBody,
              newData,
              "no_three_ds",
              "automatic",
              globalState
            );
            if (!utils.should_continue_further(createConfirmData)) {
              shouldContinue = false;
            }
          }
        );

        cy.step("Retrieve Customer Payment Methods", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Customer Payment Methods"
            );
            return;
          }
          cy.listCustomerPMCallTest(globalState);
        });
      });
    }
  );
});
