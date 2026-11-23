import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import {
  cardData,
  isSurchargeServiceAvailable,
  shouldSkipExternalSurcharge,
} from "../../../utils/externalSurcharge";

let globalState;

const createPaymentIntent = () => {
  cy.createPaymentIntentTest(
    fixtures.createPaymentBody,
    cardData(globalState, "ExternalSurchargePaymentIntent"),
    "no_three_ds",
    "automatic",
    globalState
  );
  cy.paymentMethodsCallTest(globalState);
};

const eligibilityWithError = () => {
  cy.surchargeEligibilityErrorTest(
    fixtures.eligibilityCheckBody,
    cardData(globalState, "ExternalSurchargeErrorEligibility"),
    globalState
  );
};

// Needs the same setup as 42-ExternalSurchargePayments: the surcharge connector
// from 03-ConnectorCreate and a router that can reach the surcharge service.
describe("External surcharge - connector errors", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        return shouldSkipExternalSurcharge(globalState);
      })
      .then((shouldSkip) => {
        skip = shouldSkip;
        if (!skip) {
          return isSurchargeServiceAvailable(globalState);
        }
      })
      .then((available) => {
        if (!skip && !available) {
          cy.task(
            "cli_log",
            "The surcharge service is not reachable in this environment, skipping the external surcharge errors spec"
          );
          skip = true;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  after("unlink the surcharge connector and flush global state", () => {
    cy.unlinkSurchargeConnectorFromProfile(globalState);
    cy.task("setGlobalState", globalState.data);
  });

  it("Point the profile at the surcharge connector, create a customer", () => {
    cy.linkSurchargeConnectorToProfile(globalState);
    cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
  });

  context("The surcharge service rejects the billing country", () => {
    it("Create Payment Intent -> Eligibility error -> Confirm -> Retrieve Payment", () => {
      cy.step("Create Payment Intent", createPaymentIntent);

      cy.step("Eligibility Check (the error is returned, no surcharge)", () => {
        eligibilityWithError();
      });

      cy.step("Confirm Payment (it goes on with a zero surcharge)", () => {
        cy.confirmCallTest(
          fixtures.confirmBody,
          cardData(globalState, "ExternalSurchargeErrorConfirm"),
          true,
          globalState
        );
      });

      cy.step("Retrieve Payment (the error is kept)", () => {
        cy.retrievePaymentCallTest({
          globalState,
          data: cardData(globalState, "ExternalSurchargeErrorRetrieve"),
        });
      });
    });
  });

  context("Eligibility is called again", () => {
    it("Eligibility error, then success: the second result replaces the first", () => {
      cy.step("Create Payment Intent", createPaymentIntent);

      cy.step("Eligibility Check (error)", () => {
        eligibilityWithError();
      });

      cy.step("Eligibility Check again (success)", () => {
        cy.paymentsEligibilityCheck(
          fixtures.eligibilityCheckBody,
          cardData(globalState, "ExternalSurchargeEligibility"),
          globalState
        );
      });

      cy.step("Confirm Payment (the surcharge from the second call)", () => {
        cy.confirmCallTest(
          fixtures.confirmBody,
          cardData(globalState, "ExternalSurchargeConfirm"),
          true,
          globalState
        );
      });
    });
  });
});
