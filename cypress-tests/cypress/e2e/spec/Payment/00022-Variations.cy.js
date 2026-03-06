import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import step from "../../../utils/customStep";

let globalState;

describe("Corner cases", () => {
  context("[Payment] Invalid Info", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

    it(
      "[Payment] Invalid card number -> [Payment] Invalid expiry month -> [Payment] Invalid expiry year -> [Payment] Invalid card CVV -> [Payment] Invalid currency -> [Payment] Invalid capture method -> [Payment] Invalid payment method -> [Payment] Invalid `amount_to_capture` -> [Payment] Missing required params -> [Payment] return_url - too long -> [Payment] return_url - invalid format -> [Payment] mandate_id - too long",
      () => {
        cy.step("[Payment] Invalid card number", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["InvalidCardNumber"];
          const body = Cypress._.cloneDeep(fixtures.createPaymentBody);

          cy.createConfirmPaymentTest(
            body,
            data,
            "three_ds",
            "automatic",
            globalState
          );
        });

        cy.step("[Payment] Invalid expiry month", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["InvalidExpiryMonth"];
          const body = Cypress._.cloneDeep(fixtures.createPaymentBody);

          cy.createConfirmPaymentTest(
            body,
            data,
            "three_ds",
            "automatic",
            globalState
          );
        });

        cy.step("[Payment] Invalid expiry year", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["InvalidExpiryYear"];
          const body = Cypress._.cloneDeep(fixtures.createPaymentBody);

          cy.createConfirmPaymentTest(
            body,
            data,
            "three_ds",
            "automatic",
            globalState
          );
        });

        cy.step("[Payment] Invalid card CVV", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["InvalidCardCvv"];
          const body = Cypress._.cloneDeep(fixtures.createPaymentBody);

          cy.createConfirmPaymentTest(
            body,
            data,
            "three_ds",
            "automatic",
            globalState
          );
        });

        cy.step("[Payment] Invalid currency", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["InvalidCurrency"];
          const body = Cypress._.cloneDeep(fixtures.createPaymentBody);

          cy.createConfirmPaymentTest(
            body,
            data,
            "three_ds",
            "automatic",
            globalState
          );
        });

        cy.step("[Payment] Invalid capture method", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["InvalidCaptureMethod"];
          const body = Cypress._.cloneDeep(fixtures.createPaymentBody);

          cy.createConfirmPaymentTest(
            body,
            data,
            "three_ds",
            "automatic",
            globalState
          );
        });

        cy.step("[Payment] Invalid payment method", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["InvalidPaymentMethod"];
          const body = Cypress._.cloneDeep(fixtures.createPaymentBody);

          cy.createConfirmPaymentTest(
            body,
            data,
            "three_ds",
            "automatic",
            globalState
          );
        });

        cy.step("[Payment] Invalid `amount_to_capture`", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["InvalidAmountToCapture"];
          const body = Cypress._.cloneDeep(fixtures.createPaymentBody);

          cy.createConfirmPaymentTest(
            body,
            data,
            "three_ds",
            "automatic",
            globalState
          );
        });

        cy.step("[Payment] Missing required params", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MissingRequiredParam"];
          const body = Cypress._.cloneDeep(fixtures.createPaymentBody);

          cy.createConfirmPaymentTest(
            body,
            data,
            "three_ds",
            "automatic",
            globalState
          );
        });

        cy.step("[Payment] return_url - too long", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "return_url_variations"
          ]["return_url_too_long"];
          const body = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);

          cy.createConfirmPaymentTest(
            body,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );
        });

        cy.step("[Payment] return_url - invalid format", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "return_url_variations"
          ]["return_url_invalid_format"];
          const body = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);

          cy.createConfirmPaymentTest(
            body,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );
        });

        cy.step("[Payment] mandate_id - too long", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "mandate_id_too_long"
          ];
          const body = Cypress._.cloneDeep(fixtures.createConfirmPaymentBody);

          cy.createConfirmPaymentTest(
            body,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );
        });
      }
    );
  });

  context("[Payment] Confirm w/o PMD", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    it("Create Payment Intent -> Confirm Payment w/o PMD", () => {
      let shouldContinue = true;

    it("Create payment intent -> Confirm payment intent", () => {
      let shouldContinue = true;

      cy.step("Create payment intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];
        const paymentIntentBody = Cypress._.cloneDeep(
          fixtures.createPaymentBody
        );

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

      cy.step("Confirm payment intent", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm payment intent");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntentErrored"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
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

    it(
      "Create payment intent and confirm -> Retrieve payment -> Capture call",
      () => {
        let shouldContinue = true;

        cy.step("Create payment intent and confirm", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];
          const paymentCreateConfirmBody = Cypress._.cloneDeep(
            fixtures.createConfirmPaymentBody
          );

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

        cy.step("Retrieve payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Capture call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Capture call");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["CaptureGreaterAmount"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);
        });
      }
    );
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

    it(
      "Create payment intent and confirm -> Retrieve payment -> Capture call",
      () => {
        let shouldContinue = true;

        cy.step("Create payment intent and confirm", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          const paymentCreateConfirmBody = Cypress._.cloneDeep(
            fixtures.createConfirmPaymentBody
          );

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

        cy.step("Retrieve payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Capture call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Capture call");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["CaptureCapturedAmount"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);
        });
      }
    );
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

    it(
      "Create payment intent and confirm -> Retrieve payment -> Confirm call",
      () => {
        let shouldContinue = true;

        cy.step("Create payment intent and confirm", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          const paymentCreateConfirmBody = Cypress._.cloneDeep(
            fixtures.createConfirmPaymentBody
          );

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

        cy.step("Retrieve payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm call");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["ConfirmSuccessfulPayment"];

          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        });
      }
    );
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

    it(
      "Create payment intent and confirm -> Retrieve payment -> Void call",
      () => {
        let shouldContinue = true;

        cy.step("Create payment intent and confirm", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          const paymentCreateConfirmBody = Cypress._.cloneDeep(
            fixtures.createConfirmPaymentBody
          );

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

        cy.step("Retrieve payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Void call", () => {
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
      }
    );
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

    it(
      "Create payment intent and confirm -> Retrieve payment -> Handle redirection -> Retrieve payment -> Capture call",
      () => {
        let shouldContinue = true;

        cy.step("Create payment intent and confirm", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSManualCapture"];
          const paymentCreateConfirmBody = Cypress._.cloneDeep(
            fixtures.createConfirmPaymentBody
          );

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

        cy.step("Retrieve payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle redirection");
            return;
          }
          const expected_redirection = fixtures.confirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        cy.step("Retrieve payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["3DSManualCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Capture call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Capture call");
            return;
          }
          const data = getConnectorDetails(globalState.get("commons"))[
            "card_pm"
          ]["CaptureGreaterAmount"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);
        });
      }
    );
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

    it(
      "Create payment intent and confirm -> Retrieve payment -> Refund call",
      () => {
        let shouldContinue = true;

        cy.step("Create payment intent and confirm", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          const paymentCreateConfirmBody = Cypress._.cloneDeep(
            fixtures.createConfirmPaymentBody
          );

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

        cy.step("Retrieve payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Refund call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Refund call");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["RefundGreaterAmount"];

          cy.refundCallTest(fixtures.refundBody, data, globalState);
        });
      }
    );
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

    it(
      "Create payment intent and confirm -> Retrieve payment -> Refund call",
      () => {
        let shouldContinue = true;

        cy.step("Create payment intent and confirm", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          const paymentCreateConfirmBody = Cypress._.cloneDeep(
            fixtures.createConfirmPaymentBody
          );

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

        cy.step("Retrieve payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Refund call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Refund call");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["RefundGreaterAmount"];

          cy.refundCallTest(fixtures.refundBody, data, globalState);
        });
      }
    );
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

    it(
      "No 3DS CIT -> cit-capture-call-test -> Retrieve payment -> Confirm No 3DS MIT",
      () => {
        let shouldContinue = true;

        cy.step("No 3DS CIT", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MandateSingleUseNo3DSManualCapture"];

          cy.citForMandatesCallTest(
            fixtures.citConfirmBody,
            data,
            6000,
            true,
            "manual",
            "new_mandate",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("cit-capture-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: cit-capture-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.captureCallTest(fixtures.captureBody, data, globalState);

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["Capture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Confirm No 3DS MIT", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm No 3DS MIT");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["MITAutoCapture"];

          cy.mitForMandatesCallTest(
            fixtures.mitConfirmBody,
            data,
            60000,
            true,
            "manual",
            globalState
          );
        });
      }
    );
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
      let shouldContinue = true;

      cy.step("create-payment-call-test", () => {
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

      cy.step("Confirm No 3DS", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm No 3DS");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["No3DSFailPayment"];

        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      });
    });
  });

  context("Duplicate Payment ID", () => {
    it(
      "Create new payment -> Retrieve payment -> Create a payment with a duplicate payment ID",
      () => {
        let shouldContinue = true;

        cy.step("Create new payment", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];
          const createConfirmBody = Cypress._.cloneDeep(
            fixtures.createConfirmPaymentBody
          );

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

        cy.step("Retrieve payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step(
          "Create a payment with a duplicate payment ID",
          () => {
            if (!shouldContinue) {
              cy.task(
                "cli_log",
                "Skipping step: Create a payment with a duplicate payment ID"
              );
              return;
            }
            const data = getConnectorDetails(globalState.get("connectorId"))[
              "card_pm"
            ]["DuplicatePaymentID"];
            const createConfirmBody = Cypress._.cloneDeep(
              fixtures.createConfirmPaymentBody
            );

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
      }
    );
  });

  context("Duplicate Refund ID", () => {
    it(
      "Create new payment -> retrieve-payment-call-test -> Create new refund -> Sync refund -> Create a refund with  a duplicate refund ID",
      () => {
        let shouldContinue = true;

        cy.step("Create new payment", () => {
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

        cy.step("retrieve-payment-call-test", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: retrieve-payment-call-test");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["No3DSAutoCapture"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Create new refund", () => {
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

        cy.step("Sync refund", () => {
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

        cy.step("Create a refund with  a duplicate refund ID", () => {
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
      }
    );
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

    it(
      "Create new customer -> Create a customer with a duplicate customer ID",
      () => {
        let shouldContinue = true;

        cy.step("Create new customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step(
          "Create a customer with a duplicate customer ID",
          () => {
            if (!shouldContinue) {
              cy.task(
                "cli_log",
                "Skipping step: Create a customer with a duplicate customer ID"
              );
              return;
            }
            const customerData = fixtures.customerCreateBody;
            customerData.customer_id = globalState.get("customerId");

            cy.createCustomerCallTest(customerData, globalState);
          }
        );
      }
    );
  });

  context("Confirm Payment with Invalid Publishable Key", () => {
    it(
      "Create Payment Intent -> Confirm payment with invalid publishable key",
      () => {
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

        cy.step("Confirm payment with invalid publishable key", () => {
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
          cy.then(() => globalState.set("publishableKey", "pk_snd_invalid_key"));
          cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

          // Restore key synchronously after test
          cy.then(() => globalState.set("publishableKey", originalKey));
        });
      }
    );
  });

  context("Retrieve session token with invalid publishable key", () => {
    it(
      "Create Payment Intent -> Session call with invalid publishable key",
      () => {
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

        cy.step("Session call with invalid publishable key", () => {
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
      }
    );
  });
  });
});
