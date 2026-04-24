import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("PayPal - Post-Authentication flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "PayPal Post-Authentication flow test - Create, Confirm, and CompleteAuthorize",
    () => {
      it("Create Payment Intent -> Payment Methods Call -> Confirm Payment Intent -> Handle Redirection -> Retrieve Payment -> Complete Authorize -> Retrieve Payment after CompleteAuthorize", () => {
        let shouldContinue = true;

        cy.step("Create Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PostAuthPaymentIntent"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Payment Methods Call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Payment Methods Call");
            return;
          }
          cy.paymentMethodsCallTest(globalState);
        });

        cy.step("Confirm Payment Intent", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment Intent");
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PostAuthConfirm"];

          cy.confirmCallTest(
            fixtures.confirmBody,
            confirmData,
            true,
            globalState
          );

          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle Redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle Redirection");
            return;
          }
          const expected_redirection = fixtures.confirmBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        cy.step("Retrieve Payment after Confirmation", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after Confirmation"
            );
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PostAuthConfirm"];

          cy.retrievePaymentCallTest({ globalState, data: confirmData });

          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Complete Authorize", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Complete Authorize");
            return;
          }
          const completeAuthData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PostAuthCompleteAuthorize"];

          cy.completeAuthorizeCallTest(
            {},
            completeAuthData,
            globalState
          );

          if (!utils.should_continue_further(completeAuthData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after CompleteAuthorize", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after CompleteAuthorize"
            );
            return;
          }
          const completeAuthData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PostAuthCompleteAuthorize"];

          cy.retrievePaymentCallTest({ globalState, data: completeAuthData });
        });
      });
    }
  );

  context(
    "PayPal Post-Authentication flow test - Create and Confirm (auto-redirect)",
    () => {
      it("Create and Confirm Payment -> Handle Redirection -> Retrieve Payment -> Complete Authorize -> Retrieve Payment after CompleteAuthorize", () => {
        let shouldContinue = true;

        cy.step("Create and Confirm Payment", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PostAuthConfirm"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Handle Redirection", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Handle Redirection");
            return;
          }
          const expected_redirection =
            fixtures.createConfirmPaymentBody["return_url"];
          cy.handleRedirection(globalState, expected_redirection);
        });

        cy.step("Retrieve Payment", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Retrieve Payment");
            return;
          }
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PostAuthConfirm"];

          cy.retrievePaymentCallTest({ globalState, data });

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Complete Authorize", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Complete Authorize");
            return;
          }
          const completeAuthData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PostAuthCompleteAuthorize"];

          cy.completeAuthorizeCallTest(
            {},
            completeAuthData,
            globalState
          );

          if (!utils.should_continue_further(completeAuthData)) {
            shouldContinue = false;
          }
        });

        cy.step("Retrieve Payment after CompleteAuthorize", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Retrieve Payment after CompleteAuthorize"
            );
            return;
          }
          const completeAuthData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["PostAuthCompleteAuthorize"];

          cy.retrievePaymentCallTest({ globalState, data: completeAuthData });
        });
      });
    }
  );

  context(
    "PayPal Post-Authentication flow test - Error Case: CompleteAuthorize without authentication",
    () => {
      it("Create Payment Intent -> Confirm Payment Intent without 3DS -> Attempt CompleteAuthorize (should fail)", () => {
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

        cy.step("Payment Methods Call", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Payment Methods Call");
            return;
          }
          cy.paymentMethodsCallTest(globalState);
        });

        cy.step("Confirm Payment Intent (No 3DS)", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Confirm Payment Intent");
            return;
          }
          const confirmData = getConnectorDetails(
            globalState.get("connectorId")
          )["card_pm"]["No3DSAutoCapture"];

          cy.confirmCallTest(
            fixtures.confirmBody,
            confirmData,
            true,
            globalState
          );

          if (!utils.should_continue_further(confirmData)) {
            shouldContinue = false;
          }
        });

        cy.step("Attempt Complete Authorize (Expected Error)", () => {
          if (!shouldContinue) {
            cy.task("cli_log", "Skipping step: Attempt Complete Authorize");
            return;
          }
          
          // This step expects an error since the payment wasn't in authentication_required state
          cy.request({
            method: "POST",
            url: `${globalState.get("baseUrl")}/payments/${globalState.get("paymentID")}/complete_authorize`,
            headers: {
              "Content-Type": "application/json",
              "api-key": globalState.get("apiKey"),
            },
            failOnStatusCode: false,
            body: {},
          }).then((response) => {
            // Expect error since payment was already processed
            expect(response.status).to.be.oneOf([400, 422, 500]);
            if (response.body.error) {
              expect(response.body.error).to.have.property("code");
            }
          });
        });
      });
    }
  );
});
