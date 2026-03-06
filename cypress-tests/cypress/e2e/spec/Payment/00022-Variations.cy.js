import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import step from "../../../utils/customStep";

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
      const shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentIntentBody = Cypress._.cloneDeep(
          fixtures.createPaymentBody
        );
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
      });
    });

    it("[Payment] Invalid expiry month", () => {
      const shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentIntentBody = Cypress._.cloneDeep(
          fixtures.createPaymentBody
        );
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
      });
    });

    it("[Payment] Invalid expiry year", () => {
      const shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentIntentBody = Cypress._.cloneDeep(
          fixtures.createPaymentBody
        );
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
      });
    });

    it("[Payment] Invalid card CVV", () => {
      const shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentIntentBody = Cypress._.cloneDeep(
          fixtures.createPaymentBody
        );
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
      });
    });

    it("[Payment] Invalid currency", () => {
      const shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentIntentBody = Cypress._.cloneDeep(
          fixtures.createPaymentBody
        );
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
      });
    });

    it("[Payment] Invalid capture method", () => {
      const shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentIntentBody = Cypress._.cloneDeep(
          fixtures.createPaymentBody
        );
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
      });
    });

    it("[Payment] Invalid payment method", () => {
      const shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentIntentBody = Cypress._.cloneDeep(
          fixtures.createPaymentBody
        );
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
      });
    });

    it("[Payment] Invalid `amount_to_capture`", () => {
      const shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentIntentBody = Cypress._.cloneDeep(
          fixtures.createPaymentBody
        );
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
      });
    });

    it("[Payment] Missing required params", () => {
      const shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentIntentBody = Cypress._.cloneDeep(
          fixtures.createPaymentBody
        );
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
      });
    });

    it("[Payment] return_url - too long", () => {
      const shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
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
      });
    });

    it("[Payment] return_url - invalid format", () => {
      const shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
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
      });
    });

    it("[Payment] mandate_id - too long", () => {
      const shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
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
      });
    });
  });

  context("[Payment] Confirm w/o PMD", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    it("Create Payment Intent -> Confirm Payment w/o PMD", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
        const paymentIntentBody = Cypress._.cloneDeep(
          fixtures.createPaymentBody
        );
        const createData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        cy.createPaymentIntentTest(
          paymentIntentBody,
          createData,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(createData)) {
          shouldContinue = false;
        }
      });

      step("Confirm Payment w/o PMD", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentErrored"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
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

    it("Create and Confirm Payment Intent -> Retrieve Payment -> Capture Greater Amount", () => {
      let shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        cy.createConfirmPaymentTest(
          paymentCreateConfirmBody,
          confirmData,
          "no_three_ds",
          "manual",
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Capture Greater Amount", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CaptureGreaterAmount"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
      });
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

    it("Create and Confirm Payment Intent -> Retrieve Payment -> Capture Payment", () => {
      let shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.createConfirmPaymentTest(
          paymentCreateConfirmBody,
          confirmData,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Capture Payment", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["CaptureCapturedAmount"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
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

    it("Create and Confirm Payment Intent -> Retrieve Payment -> Re-Confirm Successful Payment", () => {
      let shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.createConfirmPaymentTest(
          paymentCreateConfirmBody,
          confirmData,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Re-Confirm Successful Payment", shouldContinue, () => {
        const reconfirmData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["ConfirmSuccessfulPayment"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          reconfirmData,
          true,
          globalState
        );
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

    it("Create and Confirm Payment Intent -> Retrieve Payment -> Void Payment", () => {
      let shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.createConfirmPaymentTest(
          paymentCreateConfirmBody,
          confirmData,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Void Payment", shouldContinue, () => {
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
        cy.voidCallTest(fixtures.voidBody, voidData, globalState);
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

    it("Create and Confirm 3DS Payment Intent -> Retrieve Payment -> Handle 3DS Redirection -> Retrieve Payment After Redirection -> Capture Greater Amount", () => {
      let shouldContinue = true;

      step("Create and Confirm 3DS Payment Intent", shouldContinue, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.createConfirmPaymentTest(
          paymentCreateConfirmBody,
          confirmData,
          "three_ds",
          "manual",
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Handle 3DS Redirection", shouldContinue, () => {
        const expected_redirection = fixtures.confirmBody["return_url"];
        cy.handleRedirection(globalState, expected_redirection);
      });

      step("Retrieve Payment After Redirection", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["3DSManualCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Capture Greater Amount", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("commons"))[
          "card_pm"
        ]["CaptureGreaterAmount"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
      });
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
      it("Create and Confirm Payment Intent -> Retrieve Payment -> Refund Greater Amount", () => {
        let shouldContinue = true;

        step("Create and Confirm Payment Intent", shouldContinue, () => {
          const paymentCreateConfirmBody = Cypress._.cloneDeep(
            fixtures.createConfirmPaymentBody
          );
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSAutoCapture"];
          cy.createConfirmPaymentTest(
            paymentCreateConfirmBody,
            confirmData,
            "no_three_ds",
            "automatic",
            globalState
          );
          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        step("Retrieve Payment", shouldContinue, () => {
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSAutoCapture"];
          cy.retrievePaymentCallTest({ globalState, data: confirmData });
          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        step("Refund Greater Amount", shouldContinue, () => {
          const refundData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["RefundGreaterAmount"];
          cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        });
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

    it("Create and Confirm Payment Intent -> Retrieve Payment -> Refund Greater Amount", () => {
      let shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const paymentCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.createConfirmPaymentTest(
          paymentCreateConfirmBody,
          confirmData,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Refund Greater Amount", shouldContinue, () => {
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["RefundGreaterAmount"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
      });
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

    it("CIT - Create Mandate -> Capture Payment -> Retrieve Payment -> MIT - Recurring Mandate with Greater Amount", () => {
      let shouldContinue = true;

      step("CIT - Create Mandate", shouldContinue, () => {
        const citData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["MandateSingleUseNo3DSManualCapture"];
        cy.citForMandatesCallTest(
          fixtures.citConfirmBody,
          citData,
          6000,
          true,
          "manual",
          "new_mandate",
          globalState
        );
        if (!utils.should_continue_further(citData)) {
          shouldContinue = false;
        }
      });

      step("Capture Payment", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.captureCallTest(fixtures.captureBody, captureData, globalState);
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment", shouldContinue, () => {
        const captureData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Capture"];
        cy.retrievePaymentCallTest({ globalState, data: captureData });
        if (!utils.should_continue_further(captureData)) {
          shouldContinue = false;
        }
      });

      step(
        "MIT - Recurring Mandate with Greater Amount",
        shouldContinue,
        () => {
          const mitData = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];
          cy.mitForMandatesCallTest(
            fixtures.mitConfirmBody,
            mitData,
            60000,
            true,
            "manual",
            globalState
          );
        }
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

    it("Create Payment Intent -> Confirm Payment - Expect Failure", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
        const createData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          createData,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(createData)) {
          shouldContinue = false;
        }
      });

      step("Confirm Payment - Expect Failure", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSFailPayment"];
        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });
    });
  });

  context("[Refund] Duplicate IDs", () => {
    it("Create and Confirm Payment Intent -> Retrieve Payment -> Create Duplicate Payment ID", () => {
      let shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const createConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.createConfirmPaymentTest(
          createConfirmBody,
          confirmData,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Create Duplicate Payment ID", shouldContinue, () => {
        const duplicateCreateConfirmBody = Cypress._.cloneDeep(
          fixtures.createConfirmPaymentBody
        );
        const duplicateData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["DuplicatePaymentID"];
        duplicateData.Request.payment_id = globalState.get("paymentID");
        cy.createConfirmPaymentTest(
          duplicateCreateConfirmBody,
          duplicateData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });
    });
  });

  context("[Refund] Refund variations", () => {
    it("Duplicate Refund ID", () => {
      let shouldContinue = true;

      step("Create and Confirm Payment Intent", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          confirmData,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Retrieve Payment", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        cy.retrievePaymentCallTest({ globalState, data: confirmData });
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Partial Refund", shouldContinue, () => {
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSAutoCapture"];
        const refundData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PartialRefund"];
        cy.refundCallTest(fixtures.refundBody, refundData, globalState);
        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
        }
      });

      step("Sync Refund", shouldContinue, () => {
        const syncRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["SyncRefund"];
        cy.syncRefundCallTest(syncRefundData, globalState);
        if (!utils.should_continue_further(syncRefundData)) {
          shouldContinue = false;
        }
      });

      step("Create Duplicate Refund ID", shouldContinue, () => {
        const duplicateRefundData = getConnectorDetails(
          globalState.get("connectorId")
        )["card_pm"]["DuplicateRefundID"];
        duplicateRefundData.Request.refund_id = globalState.get("refundId");
        cy.refundCallTest(
          fixtures.refundBody,
          duplicateRefundData,
          globalState
        );
      });
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
      const shouldContinue = true;

      step("Create Customer", shouldContinue, () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      step("Create Duplicate Customer ID", shouldContinue, () => {
        const customerData = Cypress._.cloneDeep(fixtures.customerCreateBody);
        customerData.customer_id = globalState.get("customerId");
        cy.createCustomerCallTest(customerData, globalState);
      });
    });
  });

  context("[Payment] Invalid Publishable Key", () => {
    it("Create Payment Intent -> Confirm Payment with Invalid Publishable Key", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
        const createData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          createData,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(createData)) {
          shouldContinue = false;
        }
      });

      step(
        "Confirm Payment with Invalid Publishable Key",
        shouldContinue,
        () => {
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["InvalidPublishableKey"];
          const originalKey = globalState.get("publishableKey");
          cy.then(() =>
            globalState.set("publishableKey", "pk_snd_invalid_key")
          );
          cy.confirmCallTest(
            fixtures.confirmBody,
            confirmData,
            true,
            globalState
          );
          cy.then(() => globalState.set("publishableKey", originalKey));
        }
      );
    });
  });

  context("[Payment] Session Token with Invalid Publishable Key", () => {
    it("Create Payment Intent -> Retrieve Session Token with Invalid Publishable Key", () => {
      let shouldContinue = true;

      step("Create Payment Intent", shouldContinue, () => {
        const createData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          createData,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (!utils.should_continue_further(createData)) {
          shouldContinue = false;
        }
      });

      step(
        "Retrieve Session Token with Invalid Publishable Key",
        shouldContinue,
        () => {
          const sessionData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["InvalidPublishableKey"];
          const originalKey = globalState.get("publishableKey");
          cy.then(() =>
            globalState.set("publishableKey", "pk_snd_invalid_key")
          );
          cy.sessionTokenCall(
            fixtures.sessionTokenBody,
            sessionData,
            globalState
          );
          cy.then(() => globalState.set("publishableKey", originalKey));
        }
      );
    });
  });
});
