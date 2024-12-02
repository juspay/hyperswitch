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
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidCardNumber"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid expiry month", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidExpiryMonth"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid expiry year", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidExpiryYear"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid card CVV", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidCardCvv"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid currency", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidCurrency"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid capture method", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidCaptureMethod"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid payment method", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidPaymentMethod"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid `amount_to_capture`", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "InvalidAmountToCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        req_data,
        res_data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Missing required params", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "MissingRequiredParam"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        req_data,
        res_data,
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
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "PaymentIntent"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createPaymentIntentTest(
        paymentIntentBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("Confirm payment intent", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "PaymentIntentErrored"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );
    });
  });

  context("[Payment] Capture greater amount", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("Create payment intent and confirm", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSManualCapture"
      ];

      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        req_data,
        res_data,
        "no_three_ds",
        "manual",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Capture call", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "CaptureGreaterAmount"
      ];

      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.captureCallTest(
        fixtures.captureBody,
        req_data,
        res_data,
        65000,
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("[Payment] Capture successful payment", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("Create payment intent and confirm", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Retrieve payment", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("Capture call", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "CaptureCapturedAmount"
      ];

      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.captureCallTest(
        fixtures.captureBody,
        req_data,
        res_data,
        65000,
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("[Payment] Confirm successful payment", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("Create payment intent and confirm", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Retrieve payment", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("Confirm call", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "ConfirmSuccessfulPayment"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.confirmCallTest(
        fixtures.confirmBody,
        req_data,
        res_data,
        true,
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("[Payment] Void successful payment", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("Create payment intent and confirm", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Retrieve payment", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("Void call", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Void"
      ];
      let commonData = getConnectorDetails(globalState.get("commons"))[
        "card_pm"
      ]["Void"];
      let req_data = data["Request"];
      let res_data = utils.getConnectorFlowDetails(
        data,
        commonData,
        "ResponseCustom"
      );
      cy.voidCallTest(fixtures.voidBody, req_data, res_data, globalState);

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("[Payment] 3DS with greater capture", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("Create payment intent and confirm", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "3DSManualCapture"
      ];

      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        req_data,
        res_data,
        "three_ds",
        "manual",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Retrieve payment", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("Handle redirection", () => {
      let expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("Retrieve payment", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("Capture call", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "CaptureGreaterAmount"
      ];

      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.captureCallTest(
        fixtures.captureBody,
        req_data,
        res_data,
        65000,
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("[Payment] Refund exceeds captured Amount", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("Create payment intent and confirm", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Retrieve payment", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("Refund call", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Refund"
      ];
      let commonData = getConnectorDetails(globalState.get("commons"))[
        "card_pm"
      ]["Refund"];
      let req_data = data["Request"];
      let res_data = utils.getConnectorFlowDetails(
        data,
        commonData,
        "ResponseCustom"
      );
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        65000,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("[Payment] Refund unsuccessful payment", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("Create payment intent and confirm", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

      let req_data = data["Request"];
      let res_data = data["Response"];

      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        req_data,
        res_data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Retrieve payment", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("Refund call", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Refund"
      ];
      let commonData = getConnectorDetails(globalState.get("commons"))[
        "card_pm"
      ]["Refund"];
      let req_data = data["Request"];
      let res_data = utils.getConnectorFlowDetails(
        data,
        commonData,
        "ResponseCustom"
      );
      cy.refundCallTest(
        fixtures.refundBody,
        req_data,
        res_data,
        65000,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });
  });

  context("[Payment] Recurring mandate with greater mandate amount", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    beforeEach(function () {
      if (!should_continue) {
        this.skip();
      }
    });

    it("No 3DS CIT", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "MandateSingleUseNo3DSManualCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        req_data,
        res_data,
        6500,
        true,
        "manual",
        "new_mandate",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("cit-capture-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Capture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.captureCallTest(
        fixtures.captureBody,
        req_data,
        res_data,
        6500,
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("Retrieve payment", () => {
      cy.retrievePaymentCallTest(globalState);
    });

    it("Confirm No 3DS MIT", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "MITAutoCapture"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        req_data,
        res_data,
        65000,
        true,
        "manual",
        globalState
      );
    });
  });
});
