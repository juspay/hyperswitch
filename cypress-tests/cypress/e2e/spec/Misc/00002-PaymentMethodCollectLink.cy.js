import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { connectorDetails } from "../../configs/Misc/Commons";

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
      const shouldContinue = true;

      cy.step("Create merchant account", () => {
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
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
          cy.task(
            "cli_log",
            "Skipping step: Initiate Payment Method Collect Link"
          );
          return;
        }
        cy.paymentMethodCollectLinkCreate(
          fixtures.pmCollectLinkBody,
          connectorDetails.pmCollectLinkCreate,
          globalState
        );
      });

      cy.step("Render Payment Method Collect Form", () => {
        if (!shouldContinue) {
          cy.task(
            "cli_log",
            "Skipping step: Render Payment Method Collect Form"
          );
          return;
        }
        cy.paymentMethodCollectLinkRender(
          connectorDetails.pmCollectLinkRender,
          globalState
        );
      });
    });
  });

  context("Edge Cases", () => {
    it("should handle render with invalid collect id", () => {
      const shouldContinue = true;

      cy.step("Create merchant account", () => {
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
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
          cy.task(
            "cli_log",
            "Skipping step: Set invalid collect id and attempt render"
          );
          return;
        }
        globalState.set("pmCollectId", "invalid_collect_id_12345");
        cy.paymentMethodCollectLinkRender(
          connectorDetails.pmCollectLinkRenderNotFound,
          globalState
        );
      });
    });
  });
});
