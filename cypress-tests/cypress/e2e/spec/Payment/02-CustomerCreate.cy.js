import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("Customer Create flow test", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("customer-create-call-test", () => {
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
  });

  it("customer-create-call-test with valid phone_country_code", () => {
    const customerCreateBody = {
      ...fixtures.customerCreateBody,
      phone_country_code: "+1",
    };

    cy.createCustomerCallTest(customerCreateBody, globalState);
  });

  it("customer-create-call-test with invalid phone_country_code", () => {
    const customerCreateBody = {
      ...fixtures.customerCreateBody,
      phone_country_code: "United States",
    };

    const data = {
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              'Invalid value provided:phone_country_code must be a valid country calling code (e.g. "+1"), got "United States"',
            code: "IR_07",
          },
        },
      },
    };

    cy.createCustomerCallTest(customerCreateBody, globalState, data);
  });
});
