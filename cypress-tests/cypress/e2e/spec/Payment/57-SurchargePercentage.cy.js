import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import {
  shouldIncludeConnector,
  CONNECTOR_LISTS,
} from "../../configs/Payment/Utils";

let globalState;

// Covers PR #14354: surcharge_percentage returned by an external surcharge
// processor (e.g. InterPayments) is surfaced on surcharge_details in the
// eligibility, confirm, and retrieve payment responses.
//
// Requires InterPayments credentials under the "interpayments" key in
// creds.json — the flow is skipped otherwise (see createNamedConnectorCallTest).
describe("Surcharge Percentage in Payment Response", () => {
  let specShouldSkip = false;

  before("seed global state", function () {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      const connectorId = globalState.get("connectorId");

      specShouldSkip = shouldIncludeConnector(
        connectorId,
        CONNECTOR_LISTS.INCLUDE.SURCHARGE_PERCENTAGE
      );
    });
  });

  beforeEach(function () {
    if (specShouldSkip) {
      this.skip();
    }
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "External surcharge (InterPayments) — surcharge_percentage on eligibility, confirm, and retrieve",
    () => {
      let shouldContinue = true;

      it("Create Surcharge Processor Connector (InterPayments)", () => {
        cy.createNamedConnectorCallTest(
          "surcharge_processor",
          fixtures.createConnectorBody,
          [],
          globalState,
          "interpayments",
          "interpayments_surcharge",
          "profile",
          "surchargeConnector"
        );
      });

      // calculate_external_surcharge (crates/router/src/core/payments.rs) resolves
      // the surcharge connector purely from business_profile.surcharge_connector_details
      // — if it's unset, surcharge_details silently comes back None with no error,
      // even with an active surcharge_processor connector and a valid card + billing
      // address. There's no fallback to "the merchant's only surcharge connector."
      it("Point Profile at the Surcharge Connector", () => {
        const merchantId = globalState.get("merchantId");
        const profileId = globalState.get("profileId");
        const surchargeConnectorId = globalState.get("surchargeConnectorId");

        cy.request({
          method: "POST",
          url: `${globalState.get("baseUrl")}/account/${merchantId}/business_profile/${profileId}`,
          headers: {
            "Content-Type": "application/json",
            "api-key": globalState.get("apiKey"),
          },
          body: {
            surcharge_connector_details: {
              surcharge_connector_id: surchargeConnectorId,
            },
          },
          failOnStatusCode: false,
        }).then((response) => {
          cy.task(
            "cli_log",
            "x-request-id -> " + response.headers["x-request-id"]
          );
          expect(response.status, "status_code").to.equal(200);
          expect(
            response.body.surcharge_connector_details.surcharge_connector_id,
            "surcharge_connector_details.surcharge_connector_id"
          ).to.equal(surchargeConnectorId);
        });
      });

      it("Create Customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("Create Payment Intent (off_session, setup_mandate)", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SurchargePercentagePaymentIntent"];
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

      it("Payment Methods Call", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Payment Methods Call");
          return;
        }
        cy.paymentMethodsCallTest(globalState);
      });

      it("Eligibility Check — surcharge_percentage present in surcharge_details", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Eligibility Check");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SurchargePercentageEligibility"];
        cy.paymentsEligibilityCheck(
          fixtures.eligibilityCheckBody,
          data,
          globalState
        );
      });

      it("Confirm Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SurchargePercentageConfirm"];
        cy.confirmCallTest(fixtures.confirmBody, data, true, globalState);
        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
        }
      });

      it("Retrieve Payment — surcharge_percentage persisted on surcharge_details", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["SurchargePercentageRetrieve"];
        cy.retrievePaymentCallTest({ globalState, data });
      });

      after("Delete surcharge processor connector", () => {
        cy.deleteSurchargeConnector(globalState);
      });
    }
  );
});
