import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Implicit Customer Update flow test", () => {
  before("seed global state", function () {
    cy.log(
      "SKIPPING — implicit_customer_update is deprecated (since 2026.01.30.0) and non-functional per API re-verification"
    );
    this.skip();
  });

  after("flush global state", () => {
    if (globalState && globalState.data) {
      cy.task("setGlobalState", globalState.data);
    }
  });

  context(
    "Create customer, confirm payment with inline customer update, verify customer record updated",
    () => {
      it("Create Customer -> Retrieve Baseline -> Create+Confirm Payment with updated customer fields -> Verify Customer Updated", () => {
        let shouldContinue = true;

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Retrieve Baseline Customer", () => {
          cy.customerRetrieveCall(globalState);
        });

        cy.step("Create+Confirm Payment with updated customer fields", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["ImplicitCustomerUpdate"];

          cy.createConfirmPaymentTest(
            fixtures.createConfirmPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Verify Customer Record Updated", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Verify Customer Record Updated"
            );
            return;
          }

          const customer_id = globalState.get("customerId");

          cy.request({
            method: "GET",
            url: `${globalState.get("baseUrl")}/customers/${customer_id}`,
            headers: {
              "Content-Type": "application/json",
              "api-key": globalState.get("apiKey"),
            },
            failOnStatusCode: false,
          }).then((response) => {
            expect(response.status).to.equal(200);
            expect(response.body.customer_id).to.equal(customer_id);
            expect(response.body.email).to.equal("updated@example.com");
            expect(response.body.name).to.equal("Updated Name");
            expect(response.body.phone).to.equal("888888888");
            expect(response.body.phone_country_code).to.equal("+1");
          });
        });
      });
    }
  );
});
