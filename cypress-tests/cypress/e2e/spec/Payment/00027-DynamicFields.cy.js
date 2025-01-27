import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import getConnectorDetails, * as utils from "../../configs/Payment/Utils";
import { cardCreditEnabled } from "../../configs/PaymentMethodList/Commons";

let globalState;

describe("Dynamic Fields Verification", () => {
  context("Verify the Dynamic fields for card", () => {
    before("seed global state", () => {
      cy.task("getGlobalState").then((state) => {
        globalState = new State(state);
      });
    });

    after("flush global state", () => {
      cy.task("setGlobalState", globalState.data);
    });

    context(
      "Verify the Dynamic fields - Payment without billing address",
      () => {
        let shouldContinue = true;

        beforeEach(function () {
          if (!shouldContinue) {
            this.skip();
          }
        });

        it("Create Business Profile", () => {
          cy.createBusinessProfileTest(
            fixtures.businessProfile.bpCreate,
            globalState
          );
        });

        it("connector-create-call-test", () => {
          cy.createConnectorCallTest(
            "payment_processor",
            fixtures.createConnectorBody,
            cardCreditEnabled,
            globalState
          );
        });

        it("Create Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentWithoutBilling"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );
          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        });

        it("Payment Method List", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "pm_list"
          ]["PmListResponse"]["pmListDynamicFieldWithoutBilling"];
          cy.paymentMethodListTestWithRequiredFields(data, globalState);
        });
      }
    );

    context("Verify the Dynamic fields - Payment with billing address", () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentWithBilling"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Payment Method List", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pm_list"
        ]["PmListResponse"]["pmListDynamicFieldWithBilling"];
        cy.paymentMethodListTestWithRequiredFields(data, globalState);
      });
    });

    context(
      "Verify the Dynamic fields - Payment with billing First and Last name",
      () => {
        let shouldContinue = true;

        beforeEach(function () {
          if (!shouldContinue) {
            this.skip();
          }
        });

        it("Create Payment Intent", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentWithFullName"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            data,
            "no_three_ds",
            "automatic",
            globalState
          );
          if (shouldContinue)
            shouldContinue = utils.should_continue_further(data);
        });

        it("Payment Method List", () => {
          const data = getConnectorDetails(globalState.get("connectorId"))[
            "pm_list"
          ]["PmListResponse"]["pmListDynamicFieldWithNames"];
          cy.paymentMethodListTestWithRequiredFields(data, globalState);
        });
      }
    );

    context("Verify the Dynamic fields - Payment with billing Email", () => {
      let shouldContinue = true;

      beforeEach(function () {
        if (!shouldContinue) {
          this.skip();
        }
      });

      it("Create Payment Intent", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentWithBillingEmail"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (shouldContinue)
          shouldContinue = utils.should_continue_further(data);
      });

      it("Payment Method List", () => {
        const data = getConnectorDetails(globalState.get("connectorId"))[
          "pm_list"
        ]["PmListResponse"]["pmListDynamicFieldWithEmail"];
        cy.paymentMethodListTestWithRequiredFields(data, globalState);
      });
    });
  });
});
1;
