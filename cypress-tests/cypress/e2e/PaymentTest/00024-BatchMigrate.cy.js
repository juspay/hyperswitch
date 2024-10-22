import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import getConnectorDetails from "../PaymentUtils/Utils";

let globalState;

describe("Migrate payment methods in batch", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  it("Call migrate payment methods in batch endpoint", () => {
    cy.migratePaymentMethodsInBatch(
      "../../../.github/data/batch_migrate.csv",
      globalState
    );
  });

  it("List customer payment methods", () => {
    cy.listCustomerPMCallTest(globalState);
  });

  it("Make a payment", () => {
    let data = getConnectorDetails(globalState.get("connectorId"))["card_pm"][
      "No3DSAutoCapture"
    ];
    let req_data = data["Request"];
    let res_data = data["Response"];
    cy.createConfirmPaymentTest(
      fixtures.createConfirmPaymentBody,
      req_data,
      res_data,
      "no_three_ds",
      "automatic",
      globalState
    );
  });

  it("retrieve-payment-call-test", () => {
    cy.retrievePaymentCallTest(globalState);
  });
});
