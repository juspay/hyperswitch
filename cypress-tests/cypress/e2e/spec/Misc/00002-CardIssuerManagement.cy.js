import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("Card Issuer Management", () => {
  before("seed global state and create merchant account", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      // Check if merchant account already exists
      if (!globalState.get("merchantId") || !globalState.get("apiKey")) {
        // Create merchant account using admin API key
        return cy
          .merchantCreateCallTest(fixtures.merchantCreateBody, globalState)
          .then(() => {
            // Create merchant API key for listCardIssuers calls
            return cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
          });
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Happy Path Tests", () => {
    it("should create a new card issuer", () => {
      const issuerName = `Test Issuer ${Date.now()}`;
      cy.createCardIssuer({ issuer_name: issuerName }, globalState);
    });

    it("should list card issuers", () => {
      cy.listCardIssuers("", 30, globalState);
    });

    it("should update an existing card issuer", () => {
      const newName = `Updated Issuer ${Date.now()}`;
      cy.updateCardIssuer(
        globalState.get("cardIssuerId"),
        { issuer_name: newName },
        globalState
      );
    });

    it("should list card issuers with query filter", () => {
      const query = "Updated";
      cy.listCardIssuers(query, 30, globalState);
    });
  });

  context("Negative Tests", () => {
    it("should fail to create issuer with missing required field", () => {
      cy.createCardIssuer({}, globalState);
    });

    it("should fail to create issuer with duplicate name", () => {
      const issuerName =
        globalState.get("cardIssuerName") || `Duplicate Test ${Date.now()}`;
      cy.createCardIssuer({ issuer_name: issuerName }, globalState);
    });
  });

  context("Edge Case Tests", () => {
    it("should handle listing with limit of 1", () => {
      cy.listCardIssuers("", 1, globalState);
    });

    it("should handle listing with empty query", () => {
      cy.listCardIssuers("", 50, globalState);
    });

    it("should fail to update non-existent issuer", () => {
      cy.updateCardIssuer(
        "non-existent-id-12345",
        { issuer_name: "New Name" },
        globalState
      );
    });
  });
});
