import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";
import { isMockServer } from "../../../support/mitmProxy";

let globalState;

const SUPERPOSITION_URL =
  Cypress.env("SUPERPOSITION_URL") || "http://localhost:8081";
const SUPERPOSITION_WORKSPACE_ID =
  Cypress.env("SUPERPOSITION_WORKSPACE_ID") || "dev";
const SUPERPOSITION_ORG_ID = Cypress.env("SUPERPOSITION_ORG_ID") || "localorg";
// Router polls Superposition for config changes (ROUTER__SUPERPOSITION__POLLING_INTERVAL,
// 10s in CI); give it time to pick up the toggle before/after this spec's tests run.
const SUPERPOSITION_POLL_WAIT_MS = 12000;

function setImplicitCustomerUpdate(value) {
  return cy.request({
    method: "PUT",
    url: `${SUPERPOSITION_URL}/default-config/implicit_customer_update`,
    headers: {
      "x-org-id": SUPERPOSITION_ORG_ID,
      "x-workspace": SUPERPOSITION_WORKSPACE_ID,
    },
    body: {
      value,
      change_reason: `cypress: set implicit_customer_update=${value} for ImplicitCustomerUpdate spec`,
    },
  });
}

describe("Card - Implicit Customer Update flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  // Re-enable before each test so a parallel connector's after-hook resetting the
  // flag to false (between our two tests) cannot poison the router's cached value.
  // Skipped in mock-server replay mode since those tests are skipped (no cassettes).
  beforeEach("enable implicit_customer_update in Superposition", () => {
    if (isMockServer()) return;
    setImplicitCustomerUpdate(true);
    cy.wait(SUPERPOSITION_POLL_WAIT_MS);
  });

  after("flush global state", () => {
    if (globalState && globalState.data) {
      cy.task("setGlobalState", globalState.data);
    }
  });

  after("restore implicit_customer_update to false in Superposition", () => {
    if (isMockServer()) return;
    setImplicitCustomerUpdate(false);
  });

  context(
    "Create customer, confirm payment with inline customer update, verify customer record updated",
    () => {
      it("Create Customer -> Retrieve Baseline -> Create+Confirm Payment with updated customer fields -> Verify Customer Updated", () => {
        // Skip in MITM cassette-replay mode: cassettes for this new spec have
        // not been recorded yet. The feature is covered by mandatory live tests.
        if (isMockServer()) {
          cy.task(
            "cli_log",
            `[ImplicitCustomerUpdate] skipping in mock-server replay mode for ${Cypress.env("CONNECTOR")}`
          );
          return;
        }

        let shouldContinue = true;

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Retrieve Baseline Customer", () => {
          cy.customerRetrieveCall(globalState);
        });

        cy.step("Create+Confirm Payment with updated customer fields", () => {
          const connectorId = globalState.get("connectorId");
          const data =
            getConnectorDetails(connectorId)["card_pm"][
              "ImplicitCustomerUpdate"
            ];

          // Pin to the primary merchant connector via straight-through routing so
          // spec 42's secondary stripe connector on the same profile does not get
          // selected by the router's routing algorithm.
          const body = JSON.parse(
            JSON.stringify(fixtures.createConfirmPaymentBody)
          );
          body.routing = {
            type: "single",
            data: {
              connector: Cypress.env("CONNECTOR"),
              merchant_connector_id: globalState.get("merchantConnectorId"),
            },
          };

          cy.createConfirmPaymentTest(
            body,
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
            cy.task("cli_log", "Skipping step: Verify Customer Record Updated");
            return;
          }

          cy.customerRetrieveAndAssertCall(globalState, {
            email: "updated@example.com",
            name: "Updated Name",
            phone: "888888888",
            phone_country_code: "+1",
          });
        });
      });
    }
  );

  context(
    "Create customer, confirm payment with partial inline customer update, verify only specified fields changed",
    () => {
      it("Create Customer -> Confirm Payment with partial update -> Verify only email and name changed", () => {
        if (isMockServer()) {
          cy.task(
            "cli_log",
            `[ImplicitCustomerUpdate] skipping in mock-server replay mode for ${Cypress.env("CONNECTOR")}`
          );
          return;
        }

        let shouldContinue = true;

        cy.step("Create Customer", () => {
          cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
        });

        cy.step("Retrieve Baseline Customer", () => {
          cy.customerRetrieveCall(globalState);
        });

        cy.step("Create+Confirm Payment with partial customer fields", () => {
          const connectorId = globalState.get("connectorId");
          const data =
            getConnectorDetails(connectorId)["card_pm"][
              "ImplicitCustomerUpdatePartial"
            ];

          const body = JSON.parse(
            JSON.stringify(fixtures.createConfirmPaymentBody)
          );
          body.routing = {
            type: "single",
            data: {
              connector: Cypress.env("CONNECTOR"),
              merchant_connector_id: globalState.get("merchantConnectorId"),
            },
          };

          cy.createConfirmPaymentTest(
            body,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );

          if (!utils.should_continue_further(data)) {
            shouldContinue = false;
          }
        });

        cy.step("Verify Partial Customer Record Updated", () => {
          if (!shouldContinue) {
            cy.task(
              "cli_log",
              "Skipping step: Verify Partial Customer Record Updated"
            );
            return;
          }

          cy.customerRetrieveAndAssertCall(globalState, {
            email: "partial@example.com",
            name: "Partial Name",
            phone: "999999999",
            phone_country_code: "+65",
          });
        });
      });
    }
  );
});
