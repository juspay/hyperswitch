import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, {
  CONNECTOR_LISTS,
  shouldIncludeConnector,
} from "../../configs/Payment/Utils";
import * as utils from "../../configs/Payment/Utils";

let globalState;

describe("Card - Use Billing As Payment Method Billing", () => {
  before("seed global state", function () {
    let skip = false;

    cy.task("getGlobalState")
      .then((state) => {
        globalState = new State(state);
        const connector = globalState.get("connectorId");

        if (
          shouldIncludeConnector(
            connector,
            CONNECTOR_LISTS.INCLUDE.USE_BILLING_AS_PAYMENT_METHOD_BILLING
          )
        ) {
          skip = true;
          return;
        }
      })
      .then(() => {
        if (skip) {
          this.skip();
        }
      });
  });

  afterEach("flush global state", () => {
    cy.task("setGlobalState", globalState.data);
  });

  context(
    "Enable use_billing_as_payment_method_billing and create payment",
    () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("enable-use-billing-as-payment-method-billing", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          false,
          false,
          false,
          false,
          false,
          globalState,
          "profile",
          true
        );
      });

      it("create-confirm-payment-with-billing-flag-enabled", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["UseBillingAsPaymentMethodBilling"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );

        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("retrieve-payment-call-test", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["UseBillingAsPaymentMethodBilling"];

        cy.retrievePaymentCallTest({ globalState, data });
      });
    }
  );

  context(
    "Disable use_billing_as_payment_method_billing and create payment",
    () => {
      const shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("disable-use-billing-as-payment-method-billing", () => {
        cy.UpdateBusinessProfileTest(
          fixtures.businessProfile.bpUpdate,
          false,
          false,
          false,
          false,
          false,
          globalState,
          "profile",
          false
        );
      });

      it("create-confirm-payment-with-billing-flag-disabled", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["UseBillingAsPaymentMethodBillingDisabled"];

        cy.createConfirmPaymentTest(
          fixtures.createConfirmPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });
    }
  );
});
