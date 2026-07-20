import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;
let connector;

describe("Plaid Open Banking PIS flow test", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.OPEN_BANKING_PIS
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

  context("Open Banking PIS - Create and Confirm flow", () => {
    it("Create Payment Intent -> List Payment Methods -> Confirm with OpenBankingPIS -> Post Session Tokens -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
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

      cy.step("List Payment Methods", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: List Payment Methods");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      cy.step("Confirm Payment with OpenBankingPIS", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment with OpenBankingPIS"
          );
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["OpenBankingPIS"];

        cy.confirmBankRedirectCallTest(
          fixtures.confirmBody,
          data,
          true,
          globalState
        );

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Post Session Tokens (Get Plaid Link Token)", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Post Session Tokens");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["PostSessionTokens"];

        cy.postSessionTokensCallTest(data, globalState);

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
        ]["SyncPayment"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("Open Banking PIS - Error case: Missing billing country", () => {
    it("Create Payment Intent -> Confirm without billing country -> Expect failure", () => {
      let shouldContinue = true;

      cy.step("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "open_banking_pm"
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

      cy.step("Confirm Payment without billing country", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment without billing country"
          );
          return;
        }

        const paymentId = globalState.get("paymentID");
        const clientSecret = globalState.get("clientSecret");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/payments/${paymentId}/confirm`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("publishableKey"),
          },
          body: {
            client_secret: clientSecret,
            payment_method: "open_banking",
            payment_method_type: "open_banking_pis",
            payment_method_data: {
              open_banking: {
                open_banking_pis: {},
              },
            },
            return_url: "https://example.com/return",
          },
          failOnStatusCode: false,
        }).then((response) => {
          expect(response.status).to.equal(400);
          expect(response.body.error.code).to.equal("IR_04");
          expect(response.body.error.message).to.include(
            "billing.address.country"
          );
        });
      });
    });
  });
});
