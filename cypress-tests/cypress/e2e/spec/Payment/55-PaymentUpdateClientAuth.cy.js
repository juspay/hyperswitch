import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let connector;
let globalState;

describe("Payment Update via Client Authentication Tests", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.PAYMENT_UPDATE_CLIENT_AUTH
          )
        ) {
          skip = true;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Payment Update via Client Auth - Happy Path", () => {
    it("Create Payment Intent -> Update via Client Auth -> Retrieve Payment", () => {
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

      cy.step("Update Payment via Client Authentication", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Update Payment via Client Authentication"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentUpdateClientAuth"];

        cy.paymentUpdateClientAuthTest(globalState, data);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment to Verify Update", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment to Verify Update"
          );
          return;
        }

        cy.retrievePaymentCallTest({
          globalState,
          data: {
            Configs: {
              skipBillingAssertion: true,
            },
          },
          unconfirmedPayment: true,
        });
      });
    });
  });

  context("Payment Update via Client Auth - Error Cases", () => {
    it("Handle update for non-existent payment ID", () => {
      const data = {
        Request: {
          payment_method: "card",
          payment_method_data: {
            card: {
              card_number: "4111111111111111",
              card_exp_month: "08",
              card_exp_year: "30",
              card_holder_name: "joseph Doe",
              card_cvc: "999",
            },
          },
        },
        Response: {
          status: 404,
          body: {
            error: {
              type: "invalid_request",
              code: "IR_01",
              message: "Payment not found",
            },
          },
        },
      };

      const nonExistentPaymentId = "pay_nonexistent_12345";
      globalState.set("paymentId", nonExistentPaymentId);

      cy.paymentUpdateClientAuthTest(globalState, data);
    });
  });

  context("Payment Update via Client Auth - Edge Cases", () => {
    it("Handle update with invalid card data", () => {
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

      cy.step("Attempt Update with Invalid Card Data", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Attempt Update with Invalid Card Data"
          );
          return;
        }

        const data = {
          Request: {
            payment_method: "card",
            payment_method_data: {
              card: {
                card_number: "invalid_card_number",
                card_exp_month: "99",
                card_exp_year: "1999",
                card_holder_name: "Invalid",
                card_cvc: "999",
              },
            },
          },
          Response: {
            status: 400,
            body: {
              error: {
                type: "invalid_request",
                code: "IR_01",
                message: "Invalid card data",
              },
            },
          },
        };

        cy.paymentUpdateClientAuthTest(globalState, data);
      });
    });
  });
});
