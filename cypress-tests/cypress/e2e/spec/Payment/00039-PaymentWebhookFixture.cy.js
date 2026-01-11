import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;
let expected_intent_status;

describe("Payment Webhook Tests", () => {
  before("seed global state", function () {
    let skip = true;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.PAYMENTS_WEBHOOK
          )
        ) {
          skip = false;
        }
      })
      .then(() => {
        if (!skip) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("NoThreeDS Manual payment flow test", () => {
    it("create-payment-call-test", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "PaymentIntent"
      ];

      cy.createPaymentIntentTest(
        fixtures.createPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
    });

    it("payment_methods-call-test", () => {
      cy.paymentMethodsCallTest(globalState);
    });

    it("Confirm No 3DS", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
        "No3DSAutoCapture"
      ];

      cy.confirmCallTest(fixtures.confirmBody, data, true, globalState).then(
        () => {
          expected_intent_status = globalState.get("paymentIntentStatus");
        }
      );
    });
  });

  context("Webhook Processing - Status Update & Retrieval", () => { 

    it("Update-payment_status", () => {
      cy.updatePaymentStatusTest(globalState, "pending" );
    
    });

    it("send-webhook", () => {
      cy.sendWebhookTest(globalState);
    });

    it("Retrieve Payment Call Test", () => {
      cy.retrievePaymentCallTest(
        globalState,
        null,
        false,
        1,
        expected_intent_status
      );
    });
  });
  
});
