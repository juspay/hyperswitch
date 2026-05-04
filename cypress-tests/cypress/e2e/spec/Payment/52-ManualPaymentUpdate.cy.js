import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Payment Manual Update Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Manual Payment Update - Happy Path", () => {
    it("Create Payment Intent -> Manual Update -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        // Create payment intent with manual capture method
        const paymentBody = {
          ...fixtures.createPaymentBody,
          capture_method: "manual",
        };

        cy.createPaymentIntentTest(
          paymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Manual Update Payment Attempt", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update Payment Attempt"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdate"];

        const merchantId = globalState.get("merchantId");
        const paymentId = globalState.get("paymentID");

        const manualUpdateBody = {
          merchant_id: merchantId,
          attempt_id: `${paymentId}_1`,
          attempt_status: data.Request.attempt_status,
          error_code: data.Request.error_code,
          error_message: data.Request.error_message,
        };

        cy.manualPaymentStatusUpdateTest(globalState, manualUpdateBody, data);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment to Verify Manual Update", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment to Verify Manual Update"
          );
          return;
        }

        cy.retrievePaymentCallTest({ globalState, forceSync: false, unconfirmedPayment: true });
      });
    });
  });

  context("Manual Payment Update - Status Only", () => {
    it("Create Payment Intent -> Manual Update Status Only -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        const paymentBody = {
          ...fixtures.createPaymentBody,
          capture_method: "manual",
        };

        cy.createPaymentIntentTest(
          paymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Manual Update Payment Status Only", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update Payment Status Only"
          );
          return;
        }

        const merchantId = globalState.get("merchantId");
        const paymentId = globalState.get("paymentID");

        const manualUpdateBody = {
          merchant_id: merchantId,
          attempt_id: `${paymentId}_1`,
          attempt_status: "pending",
        };

        // Legacy mode test - without data parameter
        cy.manualPaymentStatusUpdateTest(globalState, manualUpdateBody);
      });

      cy.step("Retrieve Payment to Verify Status Update", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment to Verify Status Update"
          );
          return;
        }

        cy.retrievePaymentCallTest({ globalState, forceSync: false, unconfirmedPayment: true });
      });
    });
  });

  context("Manual Payment Update - Negative Cases", () => {
    it("Create Payment Intent -> Manual Update with Invalid Attempt ID", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        const paymentBody = {
          ...fixtures.createPaymentBody,
          capture_method: "manual",
        };

        cy.createPaymentIntentTest(
          paymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Manual Update with Invalid Attempt ID", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update with Invalid Attempt ID"
          );
          return;
        }

        const merchantId = globalState.get("merchantId");
        const paymentId = globalState.get("paymentID");

        const manualUpdateBody = {
          merchant_id: merchantId,
          attempt_id: "invalid_attempt_id",
          attempt_status: "pending",
        };

        cy.request({
          method: "PUT",
          url: `${Cypress.env("BASEURL")}/payments/${paymentId}/manual-update`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("adminApiKey"),
            "X-Merchant-Id": merchantId,
          },
          body: manualUpdateBody,
          failOnStatusCode: false,
        }).then((response) => {
          // Expect 400 or 404 for invalid attempt_id
          expect(response.status).to.be.oneOf([400, 404]);
        });
      });
    });
  });

  context("Manual Payment Update - Edge Cases", () => {
    it("Create Payment Intent -> Manual Update with Custom Error -> Verify Persistence", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        const paymentBody = {
          ...fixtures.createPaymentBody,
          capture_method: "manual",
        };

        cy.createPaymentIntentTest(
          paymentBody,
          data,
          "no_three_ds",
          "manual",
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Manual Update with Custom Error Code and Message", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Manual Update with Custom Error Code and Message"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdate"];

        const merchantId = globalState.get("merchantId");
        const paymentId = globalState.get("paymentID");

        const manualUpdateBody = {
          merchant_id: merchantId,
          attempt_id: `${paymentId}_1`,
          attempt_status: data.Request.attempt_status,
          error_code: data.Request.error_code,
          error_message: data.Request.error_message,
        };

        cy.manualPaymentStatusUpdateTest(globalState, manualUpdateBody, data);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment Multiple Times to Verify Persistence", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment Multiple Times to Verify Persistence"
          );
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdate"];

        // First retrieval — forceSync disabled because the payment was never
        // confirmed (no connector), so force_sync=true would trigger IR_39
        cy.retrievePaymentCallTest({ globalState, forceSync: false, unconfirmedPayment: true }).then(() => {
          const paymentId = globalState.get("paymentID");

          // Second retrieval to confirm persistence
          cy.request({
            method: "GET",
            url: `${Cypress.env("BASEURL")}/payments/${paymentId}`,
            headers: {
              "Content-Type": "application/json",
              "api-key": globalState.get("apiKey"),
            },
          }).then((response) => {
            expect(response.status).to.eq(200);
            
            // Verify error_code and error_message persist across retrievals
            if (data.Response.body.error_code) {
              expect(response.body.error_code).to.equal(
                data.Response.body.error_code
              );
            }
            
            if (data.Response.body.error_message) {
              expect(response.body.error_message).to.equal(
                data.Response.body.error_message
              );
            }
          });
        });
      });
    });
  });
});
