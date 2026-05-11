import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Step-Up Auth payment flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connectorId = globalState.get("connectorId");

        if (
          utils.shouldIncludeConnector(
            connectorId,
            utils.CONNECTOR_LISTS.INCLUDE.STEP_UP_AUTH
          )
        ) {
          skip = true;
          return;
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

  context("Step-Up Auth setup - create auth processor and update profile", () => {
    let shouldContinue = true;

    afterEach("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("create authentication processor connector", () => {
      const createConnectorBody = { ...fixtures.createConnectorBody };
      createConnectorBody.connector_name = "netcetera";

      cy.createAuthenticationProcessorConnectorTest(
        createConnectorBody,
        globalState
      );
    });

    it("update business profile with auth connector", () => {
      if (!shouldContinue) {
        cy.task(
          "cli_log",
          "Skipping step: update business profile with auth connector"
        );
        return;
      }
      cy.updateBusinessProfileAuthConnectorTest(
        fixtures.businessProfile.bpUpdateAuthConnector,
        globalState
      );
    });
  });

  context(
    "Step-Up Auth happy path - create, confirm, authenticate and retrieve",
    () => {
      let shouldContinue = true;

      afterEach("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("create payment intent with three_ds", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["PaymentIntentOnly"];

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

      it("confirm payment with three_ds card", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: confirm payment with three_ds card");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["ConfirmPayment"];

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

      it("call 3ds authentication endpoint", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: call 3ds authentication endpoint"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["ThreeDSAuthentication"];

        cy.threeDSAuthenticationCallTest(
          fixtures.threeDSAuthenticationBody,
          data,
          globalState
        );
      });

      it("retrieve payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: retrieve payment");
          return;
        }
        cy.retrievePaymentCallTest({ globalState });
      });
    }
  );

  context(
    "Step-Up Auth negative - 3ds auth without confirmed payment",
    () => {
      let shouldContinue = true;

      afterEach("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("create payment intent with three_ds only", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["PaymentIntentOnly"];

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

      it("call 3ds authentication - should fail with IR_04", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: call 3ds authentication - should fail with IR_04"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "step_up_auth"
        ]["ThreeDSAuthenticationUnconfirmed"];

        cy.threeDSAuthenticationCallTest(
          fixtures.threeDSAuthenticationBody,
          data,
          globalState
        );
      });
    }
  );
});
