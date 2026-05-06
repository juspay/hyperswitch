import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

function updateL2L3Flag(isL2L3Enabled, globalState) {
  const apiKey = globalState.get("apiKey");
  const merchantId = globalState.get("merchantId");
  const profileId = globalState.get("profileId");

  cy.request({
    method: "POST",
    url: `${globalState.get("baseUrl")}/account/${merchantId}/business_profile/${profileId}`,
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      "api-key": apiKey,
    },
    body: {
      is_l2_l3_enabled: isL2L3Enabled,
    },
    failOnStatusCode: false,
  }).then((response) => {
    if (response.headers["x-request-id"]) {
      cy.task("cli_log", "x-request-id -> " + response.headers["x-request-id"]);
    }
    expect(response.status).to.equal(200);
  });
}

describe("L2/L3 Data Processing Tests", () => {
  before(function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);

        if (
          utils.shouldIncludeConnector(
            globalState.get("connectorId"),
            utils.CONNECTOR_LISTS.INCLUDE.L2L3DATA
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

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("L2/L3 Data - Intent+Confirm Flow", () => {
    it("Update Business Profile (L2/L3 enabled) -> Create Payment Intent -> Payment Methods Call -> Confirm Payment with L2/L3 -> Retrieve Payment", () => {
      let shouldContinue = true;

      cy.step("Update Business Profile (L2/L3 enabled)", () => {
        updateL2L3Flag(true, globalState);
      });

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

      cy.step("Confirm Payment with L2/L3 Data", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Confirm Payment with L2/L3 Data");
          return;
        }
        const confirmData = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["L2L3Data"];

        if (!utils.should_continue_further(confirmData)) {
          shouldContinue = false;
          cy.task(
            "cli_log",
            "Skipping step: Confirm Payment with L2/L3 Data (TRIGGER_SKIP)"
          );
          return;
        }

        cy.confirmCallTest(
          fixtures.confirmBody,
          confirmData,
          true,
          globalState
        );
      });

      cy.step("Retrieve Payment", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["L2L3Data"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });

  context("L2/L3 Data - Negative Test (Feature Disabled)", () => {
    it("Attempt L2/L3 payment WITHOUT is_l2_l3_enabled -> Should process without L2/L3 data", () => {
      let shouldContinue = true;

      cy.step("Update Business Profile (L2/L3 disabled)", () => {
        updateL2L3Flag(false, globalState);
      });

      cy.step("Create and Confirm Payment with L2/L3 fields", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["L2L3Data"];

        if (!utils.should_continue_further(data)) {
          shouldContinue = false;
          return;
        }

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

      cy.step("Retrieve Payment and verify", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Retrieve Payment");
          return;
        }
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["L2L3Data"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    });
  });
});
