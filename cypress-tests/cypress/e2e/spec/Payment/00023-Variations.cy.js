import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import reportErrors from "../../../utils/reportErrors";

let globalState;
let paymentIntentBody;
let paymentCreateConfirmBody;

describe("Corner cases", () => {
  context("[Payment] Invalid Info", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach("seed global state", () => {
      paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
      paymentCreateConfirmBody = Cypress._.cloneDeep(
        fixtures.createConfirmPaymentBody
      );
    });

    it("[Payment] Invalid card number", () => {
      const errorStack = [];
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["InvalidCardNumber"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });

    it("[Payment] Invalid expiry month", () => {
      const errorStack = [];
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["InvalidExpiryMonth"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });

    it("[Payment] Invalid expiry year", () => {
      const errorStack = [];
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["InvalidExpiryYear"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });

    it("[Payment] Invalid card CVV", () => {
      const errorStack = [];
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["InvalidCardCvv"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });

    it("[Payment] Invalid currency", () => {
      const errorStack = [];
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["InvalidCurrency"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });

    it("[Payment] Invalid capture method", () => {
      const errorStack = [];
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["InvalidCaptureMethod"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });

    it("[Payment] Invalid payment method", () => {
      const errorStack = [];
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["InvalidPaymentMethod"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });

    it("[Payment] Invalid `amount_to_capture`", () => {
      const errorStack = [];
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["InvalidAmountToCapture"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });

    it("[Payment] Missing required params", () => {
      const errorStack = [];
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MissingRequiredParam"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });

    it("[Payment] return_url - too long", () => {
      const errorStack = [];
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "return_url_variations"
      ]["return_url_too_long"];
      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });

    it("[Payment] return_url - invalid format", () => {
      const errorStack = [];
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "return_url_variations"
      ]["return_url_invalid_format"];
      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });

    it("[Payment] mandate_id - too long", () => {
      const errorStack = [];
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "mandate_id_too_long"
      ];
      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("[Payment] Confirm w/o PMD", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create payment intent -> Confirm payment intent", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create payment intent", errorStack, () => {
        const paymentIntentBody = Cypress._.cloneDeep(
          fixtures.createPaymentBody
        );
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          paymentIntentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Confirm payment intent", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm payment intent");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentErrored"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("[Payment] Capture greater amount", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create payment intent and confirm -> Retrieve payment -> Capture call", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create payment intent and confirm", errorStack, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.createConfirmPaymentTest(
          paymentCreateConfirmBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("Capture call", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Capture call");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CaptureGreaterAmount"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("[Payment] Capture successful payment", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create payment intent and confirm -> Retrieve payment -> Capture call", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create payment intent and confirm", errorStack, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          paymentCreateConfirmBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("Capture call", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Capture call");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CaptureCapturedAmount"];

        cy.captureCallTest(fixtures.captureBody, data, globalState);
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("[Payment] Confirm successful payment", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create payment intent and confirm -> Retrieve payment -> Confirm call", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create payment intent and confirm", errorStack, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          paymentCreateConfirmBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("Confirm call", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm call");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConfirmSuccessfulPayment"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("[Payment] Void successful payment", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create payment intent and confirm -> Retrieve payment -> Void call", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create payment intent and confirm", errorStack, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          paymentCreateConfirmBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("Void call", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Void call");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Void"];
        const commonData = getConnectorDetails(globalState.get("commons"))[
          "card_pm"
        ]["Void"];

        const newData = {
          ...data,
          Response: utils.getConnectorFlowDetails(
            data,
            commonData,
            "ResponseCustom"
          ),
        };

        cy.voidCallTest(fixtures.voidBody, newData, globalState);
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("[Payment] 3DS with greater capture", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create payment intent and confirm -> Retrieve payment -> Handle redirection -> Retrieve payment -> Capture call", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create payment intent and confirm", errorStack, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];

        cy.createConfirmPaymentTest(
          paymentCreateConfirmBody,
          data,
          "three_ds",
          "manual",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("Handle redirection", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Handle redirection");
          return;
        }
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      cy.step("Retrieve payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("Capture call", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Capture call");
          return;
        }
        const data = getConnectorDetails(globalState.get("commons"))["card_pm"][
          "CaptureGreaterAmount"
        ];

        cy.captureCallTest(fixtures.captureBody, data, globalState);
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("[Payment] Refund exceeds captured Amount", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create payment intent and confirm -> Retrieve payment -> Refund call", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create payment intent and confirm", errorStack, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          paymentCreateConfirmBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("Refund call", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund call");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RefundGreaterAmount"];

        cy.refundCallTest(fixtures.refundBody, data, globalState);
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("[Payment] Refund unsuccessful payment", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create payment intent and confirm -> Retrieve payment -> Refund call", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create payment intent and confirm", errorStack, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          paymentCreateConfirmBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("Refund call", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Refund call");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RefundGreaterAmount"];

        cy.refundCallTest(fixtures.refundBody, data, globalState);
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("Card-NoThreeDS fail payment flow test", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("create-payment-call-test -> Confirm No 3DS", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("create-payment-call-test", errorStack, () => {
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

      cy.step("Confirm No 3DS", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm No 3DS");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSFailPayment"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("Duplicate Payment ID", () => {
    it("Create new payment -> Retrieve payment -> Create a payment with a duplicate payment ID", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create new payment", errorStack, () => {
        const createConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          createConfirmBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve payment", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step(
        "Create a payment with a duplicate payment ID",
        errorStack,
        () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Create a payment with a duplicate payment ID"
            );
            return;
          }
          const createConfirmBody = Cypress._.cloneDeep(
            fixtures.createConfirmPaymentBody
          );
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["DuplicatePaymentID"];

          data.Request.payment_id = globalState.get("paymentID");

          cy.createConfirmPaymentTest(
            createConfirmBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );
        }
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("Duplicate Refund ID", () => {
    it("Create new payment -> retrieve-payment-call-test -> Create new refund -> Sync refund -> Create a refund with  a duplicate refund ID", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create new payment", errorStack, () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("retrieve-payment-call-test", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];

        cy.retrievePaymentCallTest({ globalState, data });
      });

      cy.step("Create new refund", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create new refund");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartialRefund"];

        cy.refundCallTest(fixtures.refundBody, data, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Sync refund", errorStack, () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Sync refund");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SyncRefund"];

        cy.syncRefundCallTest(data, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Create a refund with  a duplicate refund ID", errorStack, () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Create a refund with  a duplicate refund ID"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["DuplicateRefundID"];

        data.Request.refund_id = globalState.get("refundId");

        cy.refundCallTest(fixtures.refundBody, data, globalState);
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("Duplicate Customer ID", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create new customer -> Create a customer with a duplicate customer ID", () => {
      const errorStack = [];

      cy.step("Create new customer", errorStack, () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      cy.step(
        "Create a customer with a duplicate customer ID",
        errorStack,
        () => {
          const customerData = fixtures.customerCreateBody;
          customerData.customer_id = globalState.get("customerId");

          cy.createCustomerCallTest(customerData, globalState);
        }
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("Confirm Payment with Invalid Publishable Key", () => {
    it("Create Payment Intent -> Confirm payment with invalid publishable key", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create Payment Intent", errorStack, () => {
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

      cy.step(
        "Confirm payment with invalid publishable key",
        errorStack,
        () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Confirm payment with invalid publishable key"
            );
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["InvalidPublishableKey"];

          const originalKey = globalState.get("publishableKey");
          //set invalid publishable key
          cy.then(() =>
            globalState.set("publishableKey", "pk_snd_invalid_key")
          );
          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

          // Restore key synchronously after test
          cy.then(() => globalState.set("publishableKey", originalKey));
        }
      );

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });

  context("Retrieve session token with invalid publishable key", () => {
    it("Create Payment Intent -> Session call with invalid publishable key", () => {
      const errorStack = [];
      let shouldContinue = true;

      cy.step("Create Payment Intent", errorStack, () => {
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

      cy.step("Session call with invalid publishable key", errorStack, () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Session call with invalid publishable key"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["InvalidPublishableKey"];

        const originalKey = globalState.get("publishableKey");
        // set invalid publishable key
        cy.then(() => globalState.set("publishableKey", "pk_snd_invalid_key"));
        cy.sessionTokenCall(fixtures.sessionTokenBody, data, globalState);

        // Restore key synchronously after test
        cy.then(() => globalState.set("publishableKey", originalKey));
      });

      cy.then(() => {
        if (errorStack.length > 0) {
          reportErrors(errorStack);
        }
      });
    });
  });
});
