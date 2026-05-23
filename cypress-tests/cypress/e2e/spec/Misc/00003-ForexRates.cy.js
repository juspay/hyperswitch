import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

describe("Forex Rates and Currency Conversion", () => {
  let globalState;
  let skip = false;
  before("seed global state and create merchant account", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
      // Ensure baseUrl and adminApiKey are set
      if (!globalState.get("baseUrl")) {
        globalState.set(
          "baseUrl",
          Cypress.env("CYPRESS_BASEURL") || Cypress.env("BASEURL")
        );
      }
      if (!globalState.get("adminApiKey")) {
        globalState.set(
          "adminApiKey",
          Cypress.env("ADMIN_API_KEY") || Cypress.env("CYPRESS_ADMIN_API_KEY")
        );
      }
      // Skip forex tests on localhost — forex endpoints are only available on integ/sandbox
      const baseUrl = globalState.get("baseUrl");
      if (baseUrl && baseUrl.includes("localhost")) {
        skip = true;
        cy.task(
          "cli_log",
          "Skipping forex tests — localhost detected, forex endpoints not available locally"
        );
      }
      if (!skip) {
        return cy
          .merchantCreateCallTest(fixtures.merchantCreateBody, globalState)
          .then(() => {
            return cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
          })
          .then(() => {
            return cy.createCustomerCallTest(
              fixtures.customerCreateBody,
              globalState
            );
          });
      }
    });
  });

  beforeEach(function () {
    if (skip) {
      this.skip();
    }
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Happy Path Tests", () => {
    it("should retrieve forex rates with base currency", () => {
      cy.getForexRates(globalState);
    });

    it("should convert small amount from USD to EUR", () => {
      const amount = 100;
      const fromCurrency = "USD";
      const toCurrency = "EUR";
      cy.convertCurrency(amount, fromCurrency, toCurrency, globalState);
    });

    it("should convert realistic amount from USD to EUR", () => {
      const amount = 10000;
      const fromCurrency = "USD";
      const toCurrency = "EUR";
      cy.convertCurrency(amount, fromCurrency, toCurrency, globalState);
    });

    it("should convert realistic amount from EUR to GBP", () => {
      const amount = 50000;
      const fromCurrency = "EUR";
      const toCurrency = "GBP";
      cy.convertCurrency(amount, fromCurrency, toCurrency, globalState);
    });
  });

  context("Negative Tests", () => {
    it("should fail to convert with invalid source currency", () => {
      const amount = 100;
      const fromCurrency = "INVALID";
      const toCurrency = "USD";
      cy.convertCurrency(amount, fromCurrency, toCurrency, globalState);
    });

    it("should fail to convert with invalid target currency", () => {
      const amount = 100;
      const fromCurrency = "USD";
      const toCurrency = "INVALID";
      cy.convertCurrency(amount, fromCurrency, toCurrency, globalState);
    });

    it("should fail to convert with negative amount", () => {
      const amount = -100;
      const fromCurrency = "USD";
      const toCurrency = "EUR";
      cy.convertCurrency(amount, fromCurrency, toCurrency, globalState);
    });
  });

  context("Auth Failure Tests", () => {
    it("should fail to retrieve forex rates without api key", () => {
      cy.getForexRatesWithoutAuth(globalState);
    });

    it("should fail to convert currency with invalid api key", () => {
      cy.convertCurrency(100, "USD", "EUR", globalState, {
        apiKey: "invalid-api-key",
        assert: (response) => {
          expect(response.status).to.equal(401);
        },
      });
    });
  });

  context("Missing Required Params Tests", () => {
    it("should fail to convert without amount", () => {
      cy.convertCurrencyMissingParam("amount", 100, "USD", "EUR", globalState);
    });

    it("should fail to convert without from_currency", () => {
      cy.convertCurrencyMissingParam(
        "from_currency",
        100,
        "USD",
        "EUR",
        globalState
      );
    });

    it("should fail to convert without to_currency", () => {
      cy.convertCurrencyMissingParam(
        "to_currency",
        100,
        "USD",
        "EUR",
        globalState
      );
    });
  });

  context("Additional Edge Case Tests", () => {
    it("should handle float amount", () => {
      const amount = 100.5;
      const fromCurrency = "USD";
      const toCurrency = "EUR";
      cy.convertCurrency(amount, fromCurrency, toCurrency, globalState);
    });

    it("should handle maximum integer amount", () => {
      const amount = Number.MAX_SAFE_INTEGER;
      const fromCurrency = "USD";
      const toCurrency = "EUR";
      cy.convertCurrency(amount, fromCurrency, toCurrency, globalState);
    });
  });
});
