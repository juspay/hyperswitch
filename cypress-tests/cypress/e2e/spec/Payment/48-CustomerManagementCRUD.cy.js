import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";

let globalState;

describe("Customer Management CRUD Tests", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("customer-create-happy-path", () => {
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
  });

  it("customer-create-invalid-email", () => {
    const invalidCustomerBody = {
      ...fixtures.customerCreateBody,
      email: "invalid-email-format",
    };

    cy.request({
      method: "POST",
      url: `${globalState.get("baseUrl")}/customers`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      body: invalidCustomerBody,
      failOnStatusCode: false,
    }).then((response) => {
      expect(response.status).to.equal(400);
      expect(response.body.error).to.exist;
    });
  });

  it("customer-retrieve-happy-path", () => {
    cy.customerRetrieveCall(globalState, 200);
  });

  it("customer-retrieve-nonexistent", () => {
    const fakeCustomerId = "cus_nonexistent_fake_id_12345";

    cy.request({
      method: "GET",
      url: `${globalState.get("baseUrl")}/customers/${fakeCustomerId}`,
      headers: {
        "Content-Type": "application/json",
        "api-key": globalState.get("apiKey"),
      },
      failOnStatusCode: false,
    }).then((response) => {
      expect(response.status).to.equal(404);
      expect(response.body.error).to.exist;
    });
  });

  it("customer-update-happy-path", () => {
    cy.customerUpdateCall(fixtures.customerUpdateBody, globalState);
  });

  it("customer-list-with-count", () => {
    cy.customerListWithCountCallTest(globalState, 10, 0);
  });

  // it("customer-delete-happy-path", () => {
  //   cy.customerDeleteCall(globalState);
  // });

  // it("customer-retrieve-after-delete", () => {
  //   cy.customerRetrieveCall(globalState, 404);
  // });
});
