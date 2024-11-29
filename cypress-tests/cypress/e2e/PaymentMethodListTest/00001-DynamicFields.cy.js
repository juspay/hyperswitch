import apiKeyCreateBody from "../../fixtures/create-api-key-body.json";
import createConnectorBody from "../../fixtures/create-connector-body.json";
import customerCreateBody from "../../fixtures/create-customer-body.json";
import merchantCreateBody from "../../fixtures/merchant-create-body.json";
import State from "../../utils/State";
import {
  card_credit_enabled,
  create_payment_body_with_currency,
} from "../PaymentMethodListUtils/Commons";
import getConnectorDetails from "../PaymentMethodListUtils/Utils";
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
      "Verify the Dynamic fields - Passing currency USD and without billing address",
      () => {
        it("Create merchant", () => {
          cy.merchantCreateCallTest(merchantCreateBody, globalState);
        });
        it("Create api-key", () => {
          cy.apiKeyCreateTest(apiKeyCreateBody, globalState);
        });

        it("Create customer", () => {
          cy.createCustomerCallTest(customerCreateBody, globalState);
        });

        it("Create connector", () => {
          cy.createNamedConnectorCallTest(
            "payment_processor",
            createConnectorBody,
            card_credit_enabled,
            globalState,
            "cybersource",
            "cybersource_US_default"
          );
        });
        it("Create Payment Intent", () => {
          let data = getConnectorDetails("stripe")["pm_list"]["PaymentIntent"];
          let req_data = data["RequestCurrencyUSD"];
          let res_data = data["Response"];

          cy.createPaymentIntentTest(
            create_payment_body_with_currency("USD"),
            req_data,
            res_data,
            "no_three_ds",
            "automatic",
            globalState
          );
        });

        it("Payment Method List", () => {
          let data =
            getConnectorDetails("stripe")["pm_list"]["PmListResponse"][
              "pmListDynamicFieldWithoutBilling"
            ];
          cy.paymentMethodListTestWithRequiredFields(data, globalState);
        });
      }
    );
    context(
      "Verify the Dynamic fields - Passing curreny USD and with billing address",
      () => {
        it("Create Payment Intent", () => {
          let data = getConnectorDetails("stripe")["pm_list"]["PaymentIntent"];
          let req_data = data["RequestCurrencyUSDWithBilling"];
          let res_data = data["Response"];

          cy.createPaymentIntentTest(
            create_payment_body_with_currency("USD"),
            req_data,
            res_data,
            "no_three_ds",
            "automatic",
            globalState
          );
        });

        it("Payment Method List", () => {
          let data =
            getConnectorDetails("stripe")["pm_list"]["PmListResponse"][
              "pmListDynamicFieldWithBilling"
            ];
          cy.paymentMethodListTestWithRequiredFields(data, globalState);
        });
      }
    );
    context("Verify the Dynamic fields - Passing First and Last name", () => {
      it("Create Payment Intent", () => {
        let data = getConnectorDetails("stripe")["pm_list"]["PaymentIntent"];
        let req_data = data["RequestWithNameField"];
        let res_data = data["Response"];

        cy.createPaymentIntentTest(
          create_payment_body_with_currency("USD"),
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("Payment Method List", () => {
        let data =
          getConnectorDetails("stripe")["pm_list"]["PmListResponse"][
            "pmListDynamicFieldWithNames"
          ];
        cy.paymentMethodListTestWithRequiredFields(data, globalState);
      });
    });
    context("Verify the Dynamic fields - Passing Customer Email", () => {
      it("Create Payment Intent", () => {
        let data = getConnectorDetails("stripe")["pm_list"]["PaymentIntent"];
        let req_data = data["RequestWithBillingEmail"];
        let res_data = data["Response"];

        cy.createPaymentIntentTest(
          create_payment_body_with_currency("USD"),
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("Payment Method List", () => {
        let data =
          getConnectorDetails("stripe")["pm_list"]["PmListResponse"][
            "pmListDynamicFieldWithEmail"
          ];
        cy.paymentMethodListTestWithRequiredFields(data, globalState);
      });
    });
  });
});
1;
