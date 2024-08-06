import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import * as utils from "../PayoutUtils/Utils";

let globalState;
let payoutBody;

describe("[Payout] Saved Card", () => {
  let should_continue = true; // variable that will be used to skip tests if a previous test fails

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      // Check if the connector supports card payouts (based on the connector configuration in creds)
      if (!globalState.get("payoutsExecution")) {
        should_continue = false;
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!should_continue) {
      this.skip();
    }
  });

  context("[Payout] [Card] Onboard customer prior to transaction", () => {
    let should_continue = true; // variable that will be used to skip tests if a previous test fails

    // This is needed to get customer payment methods
    beforeEach("seed global state", () => {
      payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
    });

    it("create customer", () => {
      cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
    });

    it("create payment method", () => {
      let data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["SavePayoutMethod"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createPaymentMethodTest(globalState, req_data, res_data);
    });

    it("list customer payment methods", () => {
      cy.listCustomerPMCallTest(globalState);
    });

    it("confirm-payout-call-with-auto-fulfill-test", () => {
      let data = utils.getConnectorDetails(globalState.get("connectorId"))[
        "card_pm"
      ]["Token"];
      let req_data = data["Request"];
      let res_data = data["Response"];
      cy.createConfirmWithTokenPayoutTest(
        payoutBody,
        req_data,
        res_data,
        true,
        true,
        globalState
      );

      if (should_continue)
        should_continue = utils.should_continue_further(res_data);
    });

    it("retrieve-payout-call-test", () => {
      cy.retrievePayoutCallTest(globalState);
    });
  });

  context(
    "[Payout] [Card] Save payment method after successful transaction",
    () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("confirm-payout-call-with-auto-fulfill-test", () => {
        let data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Fulfill"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createConfirmPayoutTest(
          payoutBody,
          req_data,
          res_data,
          true,
          true,
          globalState
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("list customer payment methods", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("confirm-payout-call-with-auto-fulfill-test", () => {
        let data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["Token"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createConfirmWithTokenPayoutTest(
          payoutBody,
          req_data,
          res_data,
          true,
          true,
          globalState
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payout-call-test", () => {
        cy.retrievePayoutCallTest(globalState);
      });
    }
  );
});

describe("[Payout] Saved Bank transfer", () => {
  let should_continue = true; // variable that will be used to skip tests if a previous test fails

  before("seed global state", () => {
    cy.task("getGlobalState").then((state) => {
      globalState = new State(state);

      // Check if the connector supports card payouts (based on the connector configuration in creds)
      if (!globalState.get("payoutsExecution")) {
        should_continue = false;
      }
    });
  });

  after("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  beforeEach(function () {
    if (!should_continue) {
      this.skip();
    }
  });

  context(
    "[Payout] [Bank Transfer] Onboard Customer Prior to Transaction",
    () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails
      beforeEach("reset payoutBody", () => {
        payoutBody = Cypress._.cloneDeep(fixtures.createPayoutBody);
      });

      it("create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("create payment method", () => {
        let data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa"]["SavePayoutMethod"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createPaymentMethodTest(globalState, req_data, res_data);
      });

      it("list customer payment methods", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("[Payout] [Bank transfer] [SEPA] Fulfill using Token", () => {
        let data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa"]["Token"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createConfirmWithTokenPayoutTest(
          payoutBody,
          req_data,
          res_data,
          true,
          true,
          globalState
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payout-call-test", () => {
        cy.retrievePayoutCallTest(globalState);
      });
    }
  );

  context(
    "[Payout] [Bank Transfer] Save payment method after successful transaction",
    () => {
      let should_continue = true; // variable that will be used to skip tests if a previous test fails

      beforeEach(function () {
        if (!should_continue) {
          this.skip();
        }
      });

      it("create customer", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("confirm-payout-call-with-auto-fulfill-test", () => {
        let data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa"]["Fulfill"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createConfirmPayoutTest(
          payoutBody,
          req_data,
          res_data,
          true,
          true,
          globalState
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("list customer payment methods", () => {
        cy.listCustomerPMCallTest(globalState);
      });

      it("[Payout] [Bank transfer] [SEPA] Fulfill using Token", () => {
        let data = utils.getConnectorDetails(globalState.get("connectorId"))[
          "bank_transfer_pm"
        ]["sepa"]["Token"];
        let req_data = data["Request"];
        let res_data = data["Response"];
        cy.createConfirmWithTokenPayoutTest(
          payoutBody,
          req_data,
          res_data,
          true,
          true,
          globalState
        );

        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("retrieve-payout-call-test", () => {
        cy.retrievePayoutCallTest(globalState);
      });
    }
  );
});
