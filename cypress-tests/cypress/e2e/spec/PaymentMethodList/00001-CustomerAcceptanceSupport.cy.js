import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import {
  cardCreditAndAchEnabledInUs,
  openBankingEnabledInGb,
  createPaymentBodyWithCurrency,
  createPaymentBodyWithCurrencyCountry,
} from "../../configs/PaymentMethodList/Commons";
import getConnectorDetails from "../../configs/PaymentMethodList/Utils";

let globalState;

describe("Payment Method List - customer_acceptance_support field", () => {
  context(
    `
    MCA1 -> Stripe configured with credit = { country = "US" } and bank_debit.ach = { country = "US" }\n
    Payment is done with country as US and currency as USD\n
    card.credit should report "supported" (configured in customer_acceptance_support.card.credit)\n
    bank_debit.ach should report "unsupported" (bank_debit has no entry in customer_acceptance_support)\n
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
          cardCreditAndAchEnabledInUs,
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

      it("payment-method-list-call-test: card.credit is supported, bank_debit.ach is unsupported", () => {
        cy.assertCustomerAcceptanceSupport(globalState, [
          {
            paymentMethod: "card",
            paymentMethodType: "credit",
            expected: "supported",
          },
          {
            paymentMethod: "bank_debit",
            paymentMethodType: "ach",
            expected: "unsupported",
          },
        ]);
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
        cy.assertCustomerAcceptanceSupport(globalState, [
          {
            paymentMethod: "bank_redirect",
            paymentMethodType: "open_banking",
            expected: "partially_supported",
          },
        ]);
      });
    }
  );
});
