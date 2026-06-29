import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Delayed Session Token flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.DELAYED_SESSION_TOKEN
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

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Delayed session token - Apple Pay and Google Pay", () => {
    it("Create Payment Intent -> Get Delayed Session Token", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
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

      cy.step("Get Delayed Session Token", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Get Delayed Session Token");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["DelayedSessionToken"];

        cy.sessionTokenCall(
          fixtures.sessionTokenBody,
          data,
          globalState
        );
      });
    });
  });

  context("Error case - Missing client_secret", () => {
    it("Create Payment Intent -> Session Token without client_secret", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
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

      cy.step("Session Token without client_secret", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Session Token without client_secret"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "wallet_pm"
        ]["DelayedSessionTokenMissingClientSecret"];

        cy.sessionTokenCall(
          fixtures.sessionTokenBody,
          data,
          globalState
        );
      });
    });
  });

  context("Error case - Invalid payment_id", () => {
    it("Session Token with invalid payment_id", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "wallet_pm"
      ]["DelayedSessionTokenInvalidPaymentId"];

      cy.sessionTokenCall(fixtures.sessionTokenBody, data, globalState);
    });
  });
});
