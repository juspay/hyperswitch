import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";

let globalState;
let paymentIntentBody;
let paymentCreateConfirmBody;

describe("Corner cases", () => {
  // This is needed to get flush out old data
  beforeEach("seed global state", () => {
    paymentIntentBody = Cypress._.cloneDeep(fixtures.createPaymentBody);
    paymentCreateConfirmBody = Cypress._.cloneDeep(
      fixtures.createConfirmPaymentBody
    );
  });

  context("[Payment] Invalid Info", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("[Payment] Invalid card number", () => {
      const data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidCardNumber"
      ];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid expiry month", () => {
      const data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidExpiryMonth"
      ];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid expiry year", () => {
      const data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidExpiryYear"
      ];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid card CVV", () => {
      const data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidCardCvv"
      ];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid currency", () => {
      const data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidCurrency"
      ];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid capture method", () => {
      const data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidCaptureMethod"
      ];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid payment method", () => {
      const data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidPaymentMethod"
      ];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid `amount_to_capture`", () => {
      const data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidAmountToCapture"
      ];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Missing required params", () => {
      const data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "MissingRequiredParam"
      ];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
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

    it("Create payment intent", () => {
      const data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "PaymentIntent"
      ];

      cy.createPaymentIntentTest(
        paymentIntentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("Confirm payment intent", () => {
      const data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "PaymentIntentErrored"
      ];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
    });
  });

  context("[Payment] Capture greater amount", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create payment intent and confirm", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Capture call", () => {
      const data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "CaptureGreaterAmount"
      ];

      cy.captureCallTest(fixtures.captureBody, data, 65000, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("[Payment] Capture successful payment", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create payment intent and confirm", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("Capture call", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["CaptureCapturedAmount"];

      cy.captureCallTest(fixtures.captureBody, data, 65000, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("[Payment] Confirm successful payment", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create payment intent and confirm", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("Confirm call", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["ConfirmSuccessfulPayment"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("[Payment] Void successful payment", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create payment intent and confirm", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("Void call", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("[Payment] 3DS with greater capture", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create payment intent and confirm", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("Handle redirection", () => {
      const expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("Retrieve payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["3DSManualCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("Capture call", () => {
      const data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "CaptureGreaterAmount"
      ];

      cy.captureCallTest(fixtures.captureBody, data, 65000, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("[Payment] Refund exceeds captured Amount", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create payment intent and confirm", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("Refund call", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];
      const commonData = getConnectorDetails(globalState.get("commons"))[
        "card_pm"
      ]["Refund"];

      const newData = {
        ...data,
        Response: utils.getConnectorFlowDetails(
          data,
          commonData,
          "ResponseCustom"
        ),
      };

      cy.refundCallTest(fixtures.refundBody, newData, 65000, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("[Payment] Refund unsuccessful payment", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create payment intent and confirm", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("Refund call", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];
      const commonData = getConnectorDetails(globalState.get("commons"))[
        "card_pm"
      ]["Refund"];

      const newData = {
        ...data,
        Response: utils.getConnectorFlowDetails(
          data,
          commonData,
          "ResponseCustom"
        ),
      };

      cy.refundCallTest(fixtures.refundBody, newData, 65000, globalState);

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("[Payment] Recurring mandate with greater mandate amount", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("No 3DS CIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MandateSingleUseNo3DSManualCapture"];

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6500,
        true,
        "manual",
        "new_mandate",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("cit-capture-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.captureCallTest(fixtures.captureBody, data, 6500, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Capture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("Confirm No 3DS MIT", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["MITAutoCapture"];

      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        data,
        65000,
        true,
        "manual",
        globalState
      );
    });
  });
  context("Card-NoThreeDS fail payment flow test", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("create-payment-call-test", () => {
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

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Confirm No 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSFailPayment"];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("retrieve-payment-call-test", () => {
      cy.retrievePaymentCallTest(globalState);
    });
  });
});
