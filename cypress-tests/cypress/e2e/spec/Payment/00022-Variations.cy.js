import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Corner cases", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("[Payment] Invalid Info", () => {
    it("[Payment] Invalid card number", () => {
      const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["InvalidCardNumber"];
      cy.createConfirmPaymentTest(paymentIntentBody, data, "three_ds", "automatic", globalState);
    });

    it("[Payment] Invalid expiry month", () => {
      const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["InvalidExpiryMonth"];
      cy.createConfirmPaymentTest(paymentIntentBody, data, "three_ds", "automatic", globalState);
    });

    it("[Payment] Invalid expiry year", () => {
      const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["InvalidExpiryYear"];
      cy.createConfirmPaymentTest(paymentIntentBody, data, "three_ds", "automatic", globalState);
    });

    it("[Payment] Invalid card CVV", () => {
      const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["InvalidCardCvv"];
      cy.createConfirmPaymentTest(paymentIntentBody, data, "three_ds", "automatic", globalState);
    });

    it("[Payment] Invalid currency", () => {
      const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["InvalidCurrency"];
      cy.createConfirmPaymentTest(paymentIntentBody, data, "three_ds", "automatic", globalState);
    });

    it("[Payment] Invalid capture method", () => {
      const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["InvalidCaptureMethod"];
      cy.createConfirmPaymentTest(paymentIntentBody, data, "three_ds", "automatic", globalState);
    });

    it("[Payment] Invalid payment method", () => {
      const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["InvalidPaymentMethod"];
      cy.createConfirmPaymentTest(paymentIntentBody, data, "three_ds", "automatic", globalState);
    });

    it("[Payment] Invalid `amount_to_capture`", () => {
      const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["InvalidAmountToCapture"];
      cy.createConfirmPaymentTest(paymentIntentBody, data, "three_ds", "automatic", globalState);
    });

    it("[Payment] Missing required params", () => {
      const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MissingRequiredParam"];
      cy.createConfirmPaymentTest(paymentIntentBody, data, "three_ds", "automatic", globalState);
    });

    it("[Payment] return_url - too long", () => {
      const paymentCreateConfirmBody = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);
      const data = getConnectorDetails(globalState.get("connectorId"))["return_url_variations"]["return_url_too_long"];
      cy.createConfirmPaymentTest(paymentCreateConfirmBody, data, "no_three_ds", "automatic", globalState);
    });

    it("[Payment] return_url - invalid format", () => {
      const paymentCreateConfirmBody = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);
      const data = getConnectorDetails(globalState.get("connectorId"))["return_url_variations"]["return_url_invalid_format"];
      cy.createConfirmPaymentTest(paymentCreateConfirmBody, data, "no_three_ds", "automatic", globalState);
    });

    it("[Payment] mandate_id - too long", () => {
      const paymentCreateConfirmBody = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);
      const data = getConnectorDetails(globalState.get("connectorId"))["mandate_id_too_long"];
      cy.createConfirmPaymentTest(paymentCreateConfirmBody, data, "no_three_ds", "automatic", globalState);
    });
  });

  context("[Payment] Confirm w/o PMD", () => {

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    it("[Payment] Confirm w/o PMD", () => {
      const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);

      const createData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
      cy.createPaymentIntentTest(paymentIntentBody, createData, "no_three_ds", "automatic", globalState);

      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntentErrored"];
      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);
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

    it("[Payment] Capture greater amount", () => {
      const paymentCreateConfirmBody = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);

      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSManualCapture"];
      cy.createConfirmPaymentTest(paymentCreateConfirmBody, confirmData, "no_three_ds", "manual", globalState);

      if(!utils.should_continue_further(confirmData)) return;

      cy.retrievePaymentCallTest({ globalState, data: confirmData });

      const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["CaptureGreaterAmount"];
      cy.captureCallTest(fixtures.captureBody, captureData, globalState);
    });
  });

  context("[Payment] Actions on successful payment", () => {

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("[Payment] Capture successful payment", () => {
      const paymentCreateConfirmBody = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);

      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
      cy.createConfirmPaymentTest(paymentCreateConfirmBody, confirmData, "no_three_ds", "automatic", globalState);

      if(!utils.should_continue_further(confirmData)) return;

      cy.retrievePaymentCallTest({ globalState, data: confirmData });

      const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["CaptureCapturedAmount"];
      cy.captureCallTest(fixtures.captureBody, captureData, globalState);
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


    it("[Payment] Confirm successful payment", () => {
      const paymentCreateConfirmBody = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);

      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
      cy.createConfirmPaymentTest(paymentCreateConfirmBody, confirmData, "no_three_ds", "automatic", globalState);

      if(!utils.should_continue_further(confirmData)) return;

      cy.retrievePaymentCallTest({ globalState, data: confirmData });

      const reconfirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["ConfirmSuccessfulPayment"];
      cy.confirmCallTest(fixtures.confirmBody, reconfirmData, true, globalState);
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

    it("[Payment] Void successful payment", () => {
      const paymentCreateConfirmBody = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);

      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
      cy.createConfirmPaymentTest(paymentCreateConfirmBody, confirmData, "no_three_ds", "automatic", globalState);

      if(!utils.should_continue_further(confirmData)) return;

      cy.retrievePaymentCallTest({ globalState, data: confirmData });

      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Void"];
      const commonData = getConnectorDetails(globalState.get("commons"))["card_pm"]["Void"];
      const voidData = {
        ...data,
        Response: utils.getConnectorFlowDetails(data, commonData, "ResponseCustom"),
      };
      cy.voidCallTest(fixtures.voidBody, voidData, globalState);
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

    it("[Payment] 3DS with greater capture", () => {
      const paymentCreateConfirmBody = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);

      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["3DSManualCapture"];
      cy.createConfirmPaymentTest(paymentCreateConfirmBody, confirmData, "three_ds", "manual", globalState);

      if(!utils.should_continue_further(confirmData)) return;

      cy.retrievePaymentCallTest({ globalState, data: confirmData });

      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);

      cy.retrievePaymentCallTest({ globalState, data: confirmData });

      const captureData = getConnectorDetails(globalState.get("commons"))["card_pm"]["CaptureGreaterAmount"];
      cy.captureCallTest(fixtures.captureBody, captureData, globalState);
    });
  });

  context("[Payment] Refund variations", () => {

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("[Payment] Refund exceeds captured Amount", () => {
      const paymentCreateConfirmBody = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);

      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
      cy.createConfirmPaymentTest(paymentCreateConfirmBody, confirmData, "no_three_ds", "automatic", globalState);

      if(!utils.should_continue_further(confirmData)) return;

      cy.retrievePaymentCallTest({ globalState, data: confirmData });

      const refundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["RefundGreaterAmount"];
      cy.refundCallTest(fixtures.refundBody, refundData, globalState);
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

    it("[Payment] Refund unsuccessful payment", () => {
      const paymentCreateConfirmBody = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);

      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
      cy.createConfirmPaymentTest(paymentCreateConfirmBody, confirmData, "no_three_ds", "automatic", globalState);

      if(!utils.should_continue_further(confirmData)) return;

      cy.retrievePaymentCallTest({ globalState, data: confirmData });

      const refundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["RefundGreaterAmount"];
      cy.refundCallTest(fixtures.refundBody, refundData, globalState);
    });
  });

  context("[Payment] Recurring mandate with greater mandate amount", () => {

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("[Payment] Recurring mandate with greater mandate amount", () => {
      const citData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MandateSingleUseNo3DSManualCapture"];
      cy.citForMandatesCallTest(fixtures.citConfirmBody, citData, 6000, true, "manual", "new_mandate", globalState);

      if(!utils.should_continue_further(citData)) return;

      const captureData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["Capture"];
      cy.captureCallTest(fixtures.captureBody, captureData, globalState);

      cy.retrievePaymentCallTest({ globalState, data: captureData });

      const mitData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["MITAutoCapture"];
      cy.mitForMandatesCallTest(fixtures.mitConfirmBody, mitData, 60000, true, "manual", globalState);
    });
  });

  context("[Payment] No 3DS fail payment flow", () => {

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Card-NoThreeDS fail payment flow", () => {
      const createData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
      cy.createPaymentIntentTest(fixtures.createPaymentBody, createData, "no_three_ds", "automatic", globalState);

      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSFailPayment"];
      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);
    });
  });
  
  context("[Payment] Duplicate IDs", () => {
    it("Duplicate Payment ID", () => {
      const createConfirmBody = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);

      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
      cy.createConfirmPaymentTest(createConfirmBody, confirmData, "no_three_ds", "automatic", globalState);

      if(!utils.should_continue_further(confirmData)) return;

      cy.retrievePaymentCallTest({ globalState, data: confirmData });

      const duplicateCreateConfirmBody = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);
      const duplicateData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["DuplicatePaymentID"];
      duplicateData.Request.payment_id = globalState.get("paymentID");
      cy.createConfirmPaymentTest(duplicateCreateConfirmBody, duplicateData, "no_three_ds", "automatic", globalState);
    });
  });

  context("[Payment] Refund variations", () => {
    it("Duplicate Refund ID", () => {

      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["No3DSAutoCapture"];
      cy.createConfirmPaymentTest(fixtures.createConfirmPaymentBody, confirmData, "no_three_ds", "automatic", globalState);

      if(!utils.should_continue_further(confirmData)) return;

      cy.retrievePaymentCallTest({ globalState, data: confirmData });

      const refundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PartialRefund"];
      cy.refundCallTest(fixtures.refundBody, refundData, globalState);

      const syncRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["SyncRefund"];
      cy.syncRefundCallTest(syncRefundData, globalState);

      const duplicateRefundData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["DuplicateRefundID"];
      duplicateRefundData.Request.refund_id = globalState.get("refundId");
      cy.refundCallTest(fixtures.refundBody, duplicateRefundData, globalState);
    });
  });

  context("[Customer] Duplicate Customer ID", () => {
    
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Duplicate Customer ID", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);

      const customerData = Cypress._.cloneDeep(fixtures.customerCreateBody);
      customerData.customer_id = globalState.get("customerId");
      cy.createCustomerCallTest(customerData, globalState);
    });
  });

  context("[Payment] Invalid Publishable Key", () => {
    it("Confirm Payment with Invalid Publishable Key", () => {

      const createData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
      cy.createPaymentIntentTest(fixtures.createPaymentBody, createData, "no_three_ds", "automatic", globalState);

      const confirmData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["InvalidPublishableKey"];
      const originalKey = globalState.get("publishableKey");
      cy.then(() => globalState.set("publishableKey", "pk_snd_invalid_key"));
      cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState);
      cy.then(() => globalState.set("publishableKey", originalKey));
    });
  });

  context("[Payment] Session Token with Invalid Publishable Key", () => {
    it("Retrieve session token with invalid publishable key", () => {

      const createData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["PaymentIntent"];
      cy.createPaymentIntentTest(fixtures.createPaymentBody, createData, "no_three_ds", "automatic", globalState);

      const sessionData = getConnectorDetails(globalState.get("connectorId"))["card_pm"]["InvalidPublishableKey"];
      const originalKey = globalState.get("publishableKey");
      cy.then(() => globalState.set("publishableKey", "pk_snd_invalid_key"));
      cy.sessionTokenCall(fixtures.sessionTokenBody, sessionData, globalState);
      cy.then(() => globalState.set("publishableKey", originalKey));
    });
  });
});