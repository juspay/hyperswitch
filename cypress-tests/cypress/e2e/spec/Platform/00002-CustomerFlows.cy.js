import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("Platform Customer Flows", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Shared Customer Across Connected Merchants", () => {
    it("create-shared-customer-using-platform-merchant", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("verify-connected-merchant-1-can-access-shared-customer", () => {
      const savedApiKey = globalState.get("apiKey");
      globalState.set("apiKey", globalState.get("apiKey_CM1"));

      cy.customerRetrieveCall(globalState);

      cy.then(() => {
        globalState.set("apiKey", savedApiKey);
      });
    });

    it("verify-connected-merchant-2-can-access-shared-customer", () => {
      const savedApiKey = globalState.get("apiKey");
      globalState.set("apiKey", globalState.get("apiKey_CM2"));

      cy.customerRetrieveCall(globalState);

      cy.then(() => {
        globalState.set("apiKey", savedApiKey);
      });
    });
  });

  context("Standard Merchant Cannot Access Shared Customer", () => {
    it("standard-merchant-cannot-retrieve-shared-customer", () => {
      const savedApiKey = globalState.get("apiKey");
      globalState.set("apiKey", globalState.get("apiKey_SM"));

      const customerId = globalState.get("customerId");

      cy.request({
        method: "GET",
        url: `${globalState.get("baseUrl")}/customers/${customerId}`,
        headers: {
          "Content-Type": "application/json",
          "api-key": globalState.get("apiKey"),
        },
        failOnStatusCode: false,
      }).then((response) => {
        expect(response.status).to.equal(404);
      });

      cy.then(() => {
        globalState.set("apiKey", savedApiKey);
      });
    });
  });
});
