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
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["InvalidCardNumber"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentIntentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          )
        );
    });

    it("[Payment] Invalid expiry month", () => {
        const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["InvalidExpiryMonth"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentIntentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          )
        );
      });

    it("[Payment] Invalid expiry year", () => {
        const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["InvalidExpiryYear"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentIntentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          )
        );
      });

    it("[Payment] Invalid card CVV", () => {
        const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["InvalidCardCvv"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentIntentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          )
        );
      });

    it("[Payment] Invalid currency", () => {
        const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["InvalidCurrency"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentIntentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          )
        );
      });

    it("[Payment] Invalid capture method", () => {
        const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["InvalidCaptureMethod"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentIntentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          )
        );
      });

    it("[Payment] Invalid payment method", () => {
        const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["InvalidPaymentMethod"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentIntentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          )
        );
      });

    it("[Payment] Invalid `amount_to_capture`", () => {
        const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["InvalidAmountToCapture"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentIntentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          )
        );
      });

    it("[Payment] Missing required params", () => {
        const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MissingRequiredParam"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentIntentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          )
        );
      });

    it("[Payment] return_url - too long", () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "return_url_variations"
        ]["return_url_too_long"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentCreateConfirmBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          )
        );
      });

    it("[Payment] return_url - invalid format", () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "return_url_variations"
        ]["return_url_invalid_format"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentCreateConfirmBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          )
        );
      });

    it("[Payment] mandate_id - too long", () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "mandate_id_too_long"
        ];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentCreateConfirmBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          )
        );
      });
    });

  context("[Payment] Confirm w/o PMD", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

      it("Create Payment Intent + Confirm Payment w/o PMD", () => {
        const paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);

        const createData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        cy.step("Create Payment Intent", () =>
          cy.createPaymentIntentTest(
            paymentIntentBody,
            createData,
            "no_three_ds",
            "automatic",
            globalState
          )
        );

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentErrored"];
        cy.step("Confirm Payment w/o PMD", () =>
          cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
        );
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

      it("Create and Confirm Payment Intent + Retrieve Payment + Capture Greater Amount", () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentCreateConfirmBody,
            confirmData,
            "no_three_ds",
            "manual",
            globalState
          )
        );

        if (!utils.should_continue_further(confirmData)) return;

        cy.step("Retrieve Payment", () =>
          cy.retrievePaymentCallTest({ globalState, data: confirmData })
        );

        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CaptureGreaterAmount"];
        cy.step("Capture Greater Amount", () =>
          cy.captureCallTest(fixtures.captureBody, captureData, globalState)
        );
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

      it("Create and Confirm Payment Intent + Retrieve Payment + Capture Payment", () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentCreateConfirmBody,
            confirmData,
            "no_three_ds",
            "automatic",
            globalState
          )
        );

        if (!utils.should_continue_further(confirmData)) return;

        cy.step("Retrieve Payment", () =>
          cy.retrievePaymentCallTest({ globalState, data: confirmData })
        );

        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CaptureCapturedAmount"];
        cy.step("Capture Payment", () =>
          cy.captureCallTest(fixtures.captureBody, captureData, globalState)
        );
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

      it("Create and Confirm Payment Intent + Retrieve Payment + Re-Confirm Successful Payment", () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentCreateConfirmBody,
            confirmData,
            "no_three_ds",
            "automatic",
            globalState
          )
        );

        if (!utils.should_continue_further(confirmData)) return;

        cy.step("Retrieve Payment", () =>
          cy.retrievePaymentCallTest({ globalState, data: confirmData })
        );

        const reconfirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ConfirmSuccessfulPayment"];
        cy.step("Re-Confirm Successful Payment", () =>
          cy.confirmCallTest(
            fixtures.confirmBody,
            reconfirmData,
            true,
            globalState
          )
        );
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

      it("Create and Confirm Payment Intent + Retrieve Payment + Void Payment", () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentCreateConfirmBody,
            confirmData,
            "no_three_ds",
            "automatic",
            globalState
          )
        );

        if (!utils.should_continue_further(confirmData)) return;

        cy.step("Retrieve Payment", () =>
          cy.retrievePaymentCallTest({ globalState, data: confirmData })
        );

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Void"];
        const commonData = getConnectorDetails(globalState.get("commons"))[
          "card_pm"
        ]["Void"];
        const voidData = {
          ...data,
          Response: utils.getConnectorFlowDetails(
            data,
            commonData,
            "ResponseCustom"
          ),
        };
        cy.step("Void Payment", () =>
          cy.voidCallTest(fixtures.voidBody, voidData, globalState)
        );
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

      it("Create and Confirm 3DS Payment Intent + Retrieve Payment + Handle 3DS Redirection + Retrieve Payment After Redirection + Capture Greater Amount", () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.step("Create and Confirm 3DS Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentCreateConfirmBody,
            confirmData,
            "three_ds",
            "manual",
            globalState
          )
        );

        if (!utils.should_continue_further(confirmData)) return;

        cy.step("Retrieve Payment", () =>
          cy.retrievePaymentCallTest({ globalState, data: confirmData })
        );

        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.step("Handle 3DS Redirection", () =>
          cy.handleRedirection(globalState, expected_redirection)
        );

        cy.step("Retrieve Payment After Redirection", () =>
          cy.retrievePaymentCallTest({ globalState, data: confirmData })
        );

        const captureData = getConnectorDetails(globalState.get("commons"))[
          "card_pm"
        ]["CaptureGreaterAmount"];
        cy.step("Capture Greater Amount", () =>
          cy.captureCallTest(fixtures.captureBody, captureData, globalState)
        );
    });
  });

  context("Refund variations", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    context("[Refund] Refund exceeds captured Amount", () => {
      it("Create and Confirm Payment Intent + Retrieve Payment + Refund Greater Amount", () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentCreateConfirmBody,
            confirmData,
            "no_three_ds",
            "automatic",
            globalState
          )
        );

        if (!utils.should_continue_further(confirmData)) return;

        cy.step("Retrieve Payment", () =>
          cy.retrievePaymentCallTest({ globalState, data: confirmData })
        );

        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RefundGreaterAmount"];
        cy.step("Refund Greater Amount", () =>
          cy.refundCallTest(fixtures.refundBody, refundData, globalState)
        );
      });
    });
  });

  context("[Refund] Refund unsuccessful payment", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

      it("Create and Confirm Payment Intent + Retrieve Payment + Refund Greater Amount", () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            paymentCreateConfirmBody,
            confirmData,
            "no_three_ds",
            "automatic",
            globalState
          )
        );

        if (!utils.should_continue_further(confirmData)) return;

        cy.step("Retrieve Payment", () =>
          cy.retrievePaymentCallTest({ globalState, data: confirmData })
        );

        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RefundGreaterAmount"];
        cy.step("Refund Greater Amount", () =>
          cy.refundCallTest(fixtures.refundBody, refundData, globalState)
        );
    });
  });

  context("[Refund] Recurring mandate with greater mandate amount", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

      it("CIT - Create Mandate + Capture Payment + Retrieve Payment + MIT - Recurring Mandate with Greater Amount", () => {
        const citData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MandateSingleUseNo3DSManualCapture"];
        cy.step("CIT - Create Mandate", () =>
          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            citData,
            6000,
            true,
            "manual",
            "new_mandate",
            globalState
          )
        );

        if (!utils.should_continue_further(citData)) return;

        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.step("Capture Payment", () =>
          cy.captureCallTest(fixtures.captureBody, captureData, globalState)
        );

        cy.step("Retrieve Payment", () =>
          cy.retrievePaymentCallTest({ globalState, data: captureData })
        );

        const mitData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MITAutoCapture"];
        cy.step("MIT - Recurring Mandate with Greater Amount", () =>
          cy.mitForMandatesCallTest(
            fixtures.mitConfirmBody,
            mitData,
            60000,
            true,
            "manual",
            globalState
          )
        );
    });
  });

  context("[Refund] No 3DS fail payment flow", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

      it("Create Payment Intent + Confirm Payment - Expect Failure", () => {
        const createData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        cy.step("Create Payment Intent", () =>
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            createData,
            "no_three_ds",
            "automatic",
            globalState
          )
        );

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSFailPayment"];
        cy.step("Confirm Payment - Expect Failure", () =>
          cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
        );
    });
  });

  context("[Refund] Duplicate IDs", () => {
      it("Create and Confirm Payment Intent + Retrieve Payment + Create Duplicate Payment ID", () => {
        const createConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            createConfirmBody,
            confirmData,
            "no_three_ds",
            "automatic",
            globalState
          )
        );

        if (!utils.should_continue_further(confirmData)) return;

        cy.step("Retrieve Payment", () =>
          cy.retrievePaymentCallTest({ globalState, data: confirmData })
        );

        const duplicateCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const duplicateData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["DuplicatePaymentID"];
        duplicateData.Request.payment_id = globalState.get("paymentID");
        cy.step("Create Duplicate Payment ID", () =>
          cy.createConfirmPaymentTest(
            duplicateCreateConfirmBody,
            duplicateData,
            "no_three_ds",
            "automatic",
            globalState
          )
        );
    });
  });

  context("[Refund] Refund variations", () => {
    it("Duplicate Refund ID", () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.step("Create and Confirm Payment Intent", () =>
          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            confirmData,
            "no_three_ds",
            "automatic",
            globalState
          )
        );

        if (!utils.should_continue_further(confirmData)) return;

        cy.step("Retrieve Payment", () =>
          cy.retrievePaymentCallTest({ globalState, data: confirmData })
        );

        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartialRefund"];
        cy.step("Partial Refund", () =>
          cy.refundCallTest(fixtures.refundBody, refundData, globalState)
        );

        if (!utils.should_continue_further(confirmData)) return;

        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.step("Sync Refund", () =>
          cy.syncRefundCallTest(syncRefundData, globalState)
        );

        const duplicateRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["DuplicateRefundID"];
        duplicateRefundData.Request.refund_id = globalState.get("refundId");
        cy.step("Create Duplicate Refund ID", () =>
          cy.refundCallTest(fixtures.refundBody, duplicateRefundData, globalState)
        );
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
        cy.step("Create Customer", () =>
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState)
        );

        const customerData = Cypress._.cloneDeep(fixtures.customerCreateBody);
        customerData.customer_id = globalState.get("customerId");
        cy.step("Create Duplicate Customer ID", () =>
          cy.createCustomerCallTest(customerData, globalState)
        );
    });
  });

  context("[Payment] Invalid Publishable Key", () => {
      it("Create Payment Intent + Confirm Payment with Invalid Publishable Key", () => {
        const createData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        cy.step("Create Payment Intent", () =>
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            createData,
            "no_three_ds",
            "automatic",
            globalState
          )
        );

        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["InvalidPublishableKey"];
        const originalKey = globalState.get("publishableKey");
        cy.then(() => globalState.set("publishableKey", "pk_snd_invalid_key"));
        cy.step("Confirm Payment with Invalid Publishable Key", () =>
          cy.confirmCallTest(fixtures.confirmBody, confirmData, true, globalState)
        );
        cy.then(() => globalState.set("publishableKey", originalKey));
    });
  });

  context("[Payment] Session Token with Invalid Publishable Key", () => {
      it("Create Payment Intent + Retrieve Session Token with Invalid Publishable Key", () => {
        const createData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        cy.step("Create Payment Intent", () =>
          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            createData,
            "no_three_ds",
            "automatic",
            globalState
          )
        );

        const sessionData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["InvalidPublishableKey"];
        const originalKey = globalState.get("publishableKey");
        cy.then(() => globalState.set("publishableKey", "pk_snd_invalid_key"));
        cy.step("Retrieve Session Token with Invalid Publishable Key", () =>
          cy.sessionTokenCall(fixtures.sessionTokenBody, sessionData, globalState)
        );
        cy.then(() => globalState.set("publishableKey", originalKey));
          });
  });
});
