import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import { customerCreateResponse } from "../../configs/Payment/Commons";

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

    cy.createCustomerCallTest(
      customerCreateBody,
      globalState,
      customerCreateResponse.InvalidPhoneCountryCode
    );
  });
});
