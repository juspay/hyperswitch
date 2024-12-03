import * as fixtures from "../../fixtures/imports";
import State from "../../utils/State";
import { card_credit_enabled } from "../PaymentMethodListUtils/Commons";
import getConnectorDetails, * as utils from "../PaymentUtils/Utils";
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
        let should_continue = true;

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
            card_credit_enabled,
            globalState
          );
        });

        it("Create Payment Intent", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentWithoutBilling"];
          let req_data = data["Request"];
          let res_data = data["Response"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            req_data,
            res_data,
            "no_three_ds",
            "automatic",
            globalState
          );
          if (should_continue)
            should_continue = utils.should_continue_further(res_data);
        });

        it("Payment Method List", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "pm_list"
          ]["PmListResponse"]["pmListDynamicFieldWithoutBilling"];
          cy.paymentMethodListTestWithRequiredFields(data, globalState);
        });
      }
    );

    context("Verify the Dynamic fields - Payment with billing address", () => {
      let should_continue = true;

      it("Create Payment Intent", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentWithBilling"];
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Payment Method List", () => {
        let should_continue = true;
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "pm_list"
        ]["PmListResponse"]["pmListDynamicFieldWithBilling"];
        cy.paymentMethodListTestWithRequiredFields(data, globalState);
      });
    });

    context(
      "Verify the Dynamic fields - Payment with billing First and Last name",
      () => {
        let should_continue = true;

        it("Create Payment Intent", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "card_pm"
          ]["PaymentWithFullName"];
          let req_data = data["Request"];
          let res_data = data["Response"];

          cy.createPaymentIntentTest(
            fixtures.createPaymentBody,
            req_data,
            res_data,
            "no_three_ds",
            "automatic",
            globalState
          );
          if (should_continue)
            should_continue = utils.should_continue_further(res_data);
        });

        it("Payment Method List", () => {
          let data = getConnectorDetails(globalState.get("connectorId"))[
            "pm_list"
          ]["PmListResponse"]["pmListDynamicFieldWithNames"];
          cy.paymentMethodListTestWithRequiredFields(data, globalState);
        });
      }
    );

    context("Verify the Dynamic fields - Payment with billing Email", () => {
      let should_continue = true;

      it("Create Payment Intent", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "card_pm"
        ]["PaymentWithBillingEmail"];
        let req_data = data["Request"];
        let res_data = data["Response"];

        cy.createPaymentIntentTest(
          fixtures.createPaymentBody,
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
        if (should_continue)
          should_continue = utils.should_continue_further(res_data);
      });

      it("Payment Method List", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "pm_list"
        ]["PmListResponse"]["pmListDynamicFieldWithEmail"];
        cy.paymentMethodListTestWithRequiredFields(data, globalState);
      });
    });
  });
});
1;