import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import {
  cardCreditEnabledInUs,
  openBankingEnabledInGb,
  createPaymentBodyWithCurrency,
  createPaymentBodyWithCurrencyCountry,
} from "../../configs/PaymentMethodList/Commons";
import getConnectorDetails from "../../configs/PaymentMethodList/Utils";

let globalState;
let pristineGlobalStateData;

describe("Payment Method List - customer_acceptance_support field", () => {
  before("snapshot global state before creating throwaway merchants", () => {
    cy.task("getGlobalState").then((state) => {
      pristineGlobalStateData = JSON.parse(JSON.stringify(state));
    });
  });

  after("restore global state for specs running after this one", () => {
    cy.task("setGlobalState", pristineGlobalStateData);
  });

  context(
    `
    MCA1 -> Stripe configured with credit = { country = "US" }\n
    Payment is done with country as US and currency as USD\n
    card.credit should report "supported" (configured in customer_acceptance_support.card.credit)\n
    `,
    () => {
      before("seed global state", () => {
        cy.task("getGlobalState").then((state) => {
          globalState = new State(state);
        });
      });

      after("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("merchant-create-call-test", () => {
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditEnabledInUs,
          globalState,
          "stripe",
          "stripe_US_default"
        );
      });

      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: data.RequestCurrencyUSD,
          RequestCurrencyUSD: undefined,
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrency("USD"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("payment-method-list-call-test: card.credit is supported", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PmListResponse"][
            "PmListCustomerAcceptanceSupportCardCredit"
          ];
        cy.paymentMethodsCallTest(globalState, data);
      });
    }
  );

  context(
    `
    MCA2 -> Volt configured with bank_redirect.open_banking = { country = "GB", currency = "GBP" }\n
    Payment is done with country as GB and currency as GBP\n
    bank_redirect.open_banking should report "partially_supported" (configured in customer_acceptance_support.bank_redirect.open_banking)\n
    `,
    () => {
      before("seed global state", () => {
        cy.task("getGlobalState").then((state) => {
          globalState = new State(state);
        });
      });

      after("flush global state", () => {
        cy.task("setGlobalState", globalState.data);
      });

      it("merchant-create-call-test", () => {
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });

      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          openBankingEnabledInGb,
          globalState,
          "volt",
          "volt_GB_default"
        );
      });

      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: {
            currency: "GBP",
            customer_acceptance: null,
            setup_future_usage: "off_session",
            authentication_type: "no_three_ds",
          },
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrencyCountry("GBP", "GB", "GB"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      it("payment-method-list-call-test: bank_redirect.open_banking is partially_supported", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PmListResponse"][
            "PmListCustomerAcceptanceSupportOpenBanking"
          ];
        cy.paymentMethodsCallTest(globalState, data);
      });
    }
  );
});
