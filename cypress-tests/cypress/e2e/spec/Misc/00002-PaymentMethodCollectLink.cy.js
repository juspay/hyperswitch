import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("Payment Method Collect Link", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Happy Path Flow", () => {
    it("should create merchant, customer, payment method collect link, and render form", () => {
      let shouldContinue = true;

      cy.step("Create merchant account", () => {
        cy.merchantCreateCallTest(
          fixtures.merchantCreateBody,
          globalState
        );
      });

      cy.step("Create merchant API key", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create merchant API key");
          return;
        }
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });

      cy.step("Create customer", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create customer");
          return;
        }
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      cy.step("Initiate Payment Method Collect Link", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Initiate Payment Method Collect Link");
          return;
        }
        const data = {
          Request: {
            return_url: "https://example.com/return",
          },
          Response: {
            status: 200,
            body: {
              status: "pending",
            },
          },
        };
        cy.paymentMethodCollectLinkCreate(fixtures.pmCollectLinkBody, data, globalState);
      });

      cy.step("Render Payment Method Collect Form", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Render Payment Method Collect Form");
          return;
        }
        const data = {
          Response: {
            status: 200,
          },
        };
        cy.paymentMethodCollectLinkRender(data, globalState);
      });
    });
  });

  context("Edge Cases", () => {
    it("should handle render with invalid collect id", () => {
      let shouldContinue = true;

      cy.step("Create merchant account", () => {
        cy.merchantCreateCallTest(
          fixtures.merchantCreateBody,
          globalState
        );
      });

      cy.step("Create merchant API key", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Create merchant API key");
          return;
        }
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });

      cy.step("Set invalid collect id and attempt render", () => {
        if (!shouldContinue) {
          cy.task("cli_log", "Skipping step: Set invalid collect id and attempt render");
          return;
        }
        globalState.set("pmCollectId", "invalid_collect_id_12345");
        const data = {
          Response: {
            status: 404,
            body: {
              error: {
                type: "invalid_request",
                code: "IR_37",
                message: "Resource not found",
              },
            },
          },
        };
        cy.paymentMethodCollectLinkRender(data, globalState);
      });
    });
  });
});
