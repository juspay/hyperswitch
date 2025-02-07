import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Duplicate Id Handling Scenarios", () => {
  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context("Duplicate Payment ID", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });

    it("Create new payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Retrieve payment", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["No3DSAutoCapture"];

      cy.retrievePaymentCallTest(globalState, data);
    });

    it("Create a payment with a duplicate payment ID", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["DuplicatePaymentID"];

      fixtures.createConfirmPaymentBody.payment_id =
        globalState.get("paymentID");

      cy.createConfirmPaymentTest(
        fixtures.createConfirmPaymentBody,
        data,
        "no_three_ds",
        "automatic",
        globalState
      );

      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("Duplicate Refund ID", () => {
    let shouldContinue = true; // variable that will be used to skip tests if a previous test fails

    beforeEach(function () {
      if (!shouldContinue) {
        this.skip();
      }
    });
    it("Create new refund", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Refund"];

      cy.refundCallTest(fixtures.refundBody, data, 650, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Sync refund", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SyncRefund"];

      cy.syncRefundCallTest(data, globalState);
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });

    it("Create a refund with  a duplicate refund ID", () => {
      const data = getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["DuplicateRefundID"];

      fixtures.refundBody.refund_id = globalState.get("refundId");

      cy.refundCallTest(fixtures.refundBody, data, 650, globalState).then(
        (response) => {
          globalState.set("originalRefundID", response.body.refund_id);
        }
      );
      if (shouldContinue) shouldContinue = utils.should_continue_further(data);
    });
  });

  context("Duplicate Customer ID", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    it("Create new customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("Create a customer with a duplicate customer ID", () => {
      fixtures.customerCreateBody.customer_id = globalState.get("customerId");

      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState).then(
        (response) => {
          expect(response.status).to.equal(400);
          expect(response.body.error.code).to.equal("IR_12");
          expect(response.body.error.message).to.equal(
            "Customer with the given `customer_id` already exists"
          );
        }
      );
    });
  });
});
