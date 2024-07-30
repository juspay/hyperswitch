import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails from "../PaymentUtils/Utils";

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

  context("[Payment] [Payment create] Invalid Card Info", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("[Payment create] Invalid card number", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "invalidCardNumber"
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

    it("[Payment create] Invalid expiry month", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "invalidExpiryMonth"
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

    it("[Payment create] Invalid expiry year", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "invalidExpiryYear"
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

    it("[Payment create] Invalid card CVV", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
        "invalidCardCvv"
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
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create payment intent", () => {
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

    it("Create payment intent", () => {
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
    });

    it("Capture call", () => {
      let data = getConnectorDetails(globalState.get("commons"))["card_pm"][
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

    it("Create payment intent", () => {
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
    });

    it("Void call", () => {
      let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "VoidErrored"
      ];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.voidCallTest(fixtures.voidBody, req_data, res_data, globalState);
    });
  });
});
