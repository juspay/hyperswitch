import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import { validateConfig } from "../../utils/featureFlags";
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

      cy.createConfirmPaymentTest(
        paymentIntentBody,
        data,
        "three_ds",
        "automatic",
        globalState
      );
    });

    it("[Payment] Invalid expiry month", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
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
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
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
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
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
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
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
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
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
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
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
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
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
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
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
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
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
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "PaymentIntentErrored"
      ];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
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

      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        data,
        "no_three_ds",
        "manual",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("Capture call", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "CaptureGreaterAmount"
      ];

      cy.captureCallTest(fixtures.captureBody, data, 65000, globalState);

      if (should_continue)
        should_continue = utils.should_continue_further(data);
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

      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

      let configs = validateConfig(data["Configs"]);

      cy.retrievePaymentCallTest(globalState, configs);
    });

    it("Capture call", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "CaptureCapturedAmount"
      ];

      cy.captureCallTest(fixtures.captureBody, data, 65000, globalState);

      if (should_continue)
        should_continue = utils.should_continue_further(data);
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

      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

      let configs = validateConfig(data["Configs"]);

      cy.retrievePaymentCallTest(globalState, configs);
    });

    it("Confirm call", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "ConfirmSuccessfulPayment"
      ];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);

      if (should_continue)
        should_continue = utils.should_continue_further(data);
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

      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

      let configs = validateConfig(data["Configs"]);

      cy.retrievePaymentCallTest(globalState, configs);
    });

    it("Void call", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Void"
      ];
      let commonData = getConnectorDetails(globalState.get("commons"))[
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

      if (should_continue)
        should_continue = utils.should_continue_further(data);
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

      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        data,
        "three_ds",
        "manual",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "3DSManualCapture"
      ];

      let configs = validateConfig(data["Configs"]);

      cy.retrievePaymentCallTest(globalState, configs);
    });

    it("Handle redirection", () => {
      let expected_redirection = fixtures.confirmBody["return_url"];
      cy.handleRedirection(globalState, expected_redirection);
    });

    it("Retrieve payment", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "3DSManualCapture"
      ];

      let configs = validateConfig(data["Configs"]);

      cy.retrievePaymentCallTest(globalState, configs);
    });

    it("Capture call", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "CaptureGreaterAmount"
      ];

      cy.captureCallTest(fixtures.captureBody, data, 65000, globalState);

      if (should_continue)
        should_continue = utils.should_continue_further(data);
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

      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

      let configs = validateConfig(data["Configs"]);

      cy.retrievePaymentCallTest(globalState, configs);
    });

    it("Refund call", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Refund"
      ];
      let commonData = getConnectorDetails(globalState.get("commons"))[
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
      if (should_continue)
        should_continue = utils.should_continue_further(data);
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

      cy.createConfirmPaymentTest(
        paymentCreateConfirmBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

      let configs = validateConfig(data["Configs"]);

      cy.retrievePaymentCallTest(globalState, configs);
    });

    it("Refund call", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Refund"
      ];
      let commonData = getConnectorDetails(globalState.get("commons"))[
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

      if (should_continue)
        should_continue = utils.should_continue_further(data);
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

      cy.citForMandatesCallTest(
        fixtures.citConfirmBody,
        data,
        6500,
        true,
        "manual",
        "new_mandate",
        globalState
      );
      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("cit-capture-call-test", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Capture"
      ];

      cy.captureCallTest(fixtures.captureBody, data, 6500, globalState);
      if (should_continue)
        should_continue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Capture"
      ];

      let configs = validateConfig(data["Configs"]);

      cy.retrievePaymentCallTest(globalState, configs);
    });

    it("Confirm No 3DS MIT", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "Capture"
      ];
      let configs = validateConfig(data["Configs"]);

      cy.mitForMandatesCallTest(
        fixtures.mitConfirmBody,
        65000,
        true,
        "manual",
        globalState,
        configs
      );
    });
  });
});
