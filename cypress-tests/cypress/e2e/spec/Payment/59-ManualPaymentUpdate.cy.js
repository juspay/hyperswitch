import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let connector;
let globalState;

describe("Payment Manual Update Tests", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connector,
            utils.CONNECTOR_LISTS.INCLUDE.MANUAL_PAYMENT_UPDATE
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

  context("Manual Payment Update - Happy Path", () => {
    it("Create Payment Intent -> Manual Update -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
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
          cy.task("cli_log", "Skipping step: Manual Update Payment Attempt");
          return;
        }

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdate"];

        cy.manualPaymentStatusUpdateTest(globalState, data);

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

  context("Manual Payment Update - Status Only", () => {
    it("Create Payment Intent -> Manual Update Status Only -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
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

        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["ManualPaymentUpdateStatusOnly"];

        cy.manualPaymentStatusUpdateTest(globalState, data);
      });

      cy.step("Retrieve Payment to Verify Status Update", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment to Verify Status Update"
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

  context("Manual Payment Update - Negative Cases", () => {
    it("Create Payment Intent -> Manual Update with Invalid Attempt ID", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent with Manual Capture", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentIntent"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
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

        cy.manualPaymentUpdateNegativeTest(globalState, "invalid_attempt_id");
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

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
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

        cy.manualPaymentStatusUpdateTest(globalState, data);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment to Verify Persistence", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Retrieve Payment to Verify Persistence"
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
});
