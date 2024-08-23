import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";

let globalState;

describe("Autoretries ", () => {
  let should_continue = true; // variable that will be used to skip tests if a previous test fails

  beforeEach(function () {
    if (!should_continue) {
      this.skip();
    }
  });

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Payment with max retries 1", () => {

    it("retrieve-mca", () => {
      cy.ListMCAbyMID(globalState);
    });

    it("api-key-create-call-test", () => {
      cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
    });

    it("customer-create-call-test", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });
    it("set-config-enable-autoretry", () => {
      cy.enableAutoRetry(fixtures.autoRetryShouldCallGsmConfig.globalState);
    });
  });
});
