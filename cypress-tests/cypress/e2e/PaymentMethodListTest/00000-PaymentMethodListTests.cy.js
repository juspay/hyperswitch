import apiKeyCreateBody from "../../fixtures/create-api-key-body.json";
import createConnectorBody from "../../fixtures/create-connector-body.json";
import merchantCreateBody from "../../fixtures/merchant-create-body.json";
import State from "../../utils/State";
import {
  bank_redirect_ideal_and_credit_enabled,
  bank_redirect_ideal_enabled,
  card_credit_enabled,
  card_credit_enabled_in_US,
  card_credit_enabled_in_USD,
  create_payment_body_with_currency,
  create_payment_body_with_currency_country,
} from "../PaymentMethodListUtils/Commons";
import getConnectorDetails from "../PaymentMethodListUtils/Utils";

// Testing for scenario:
// MCA1 -> Stripe configured with ideal = { country = "NL", currency = "EUR" }
// MCA2 -> Cybersource configured with credit = { currency = "USD" }
// Payment is done with currency as EUR and no billing address
// The resultant Payment Method list should only have ideal with stripe

let globalState;
describe("Payment Method list using Constraint Graph flow tests", () => {
  // Testing for scenario:
  // MCA1 -> Stripe configured with ideal = { country = "NL", currency = "EUR" }
  // MCA2 -> Cybersource configured with credit = { currency = "USD" }
  // Payment is done with currency as EUR and no billing address
  // The resultant Payment Method list should only have ideal with stripe
  context(
    `
    MCA1 -> Stripe configured with ideal = { country = "NL", currency = "EUR" }\n
    MCA2 -> Cybersource configured with credit = { currency = "USD" }\n
    Payment is done with currency as EUR and no billing address\n
    The resultant Payment Method list should only have ideal with stripe\n
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
        cy.merchantCreateCallTest(merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(apiKeyCreateBody, globalState);
      });

      // stripe connector create with ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          createConnectorBody,
          bank_redirect_ideal_enabled,
          globalState,
          "stripe"
        );
      });

      // cybersource connector create with card credit enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          createConnectorBody,
          card_credit_enabled,
          globalState,
          "cybersource"
        );
      });

      // creating payment with currency as EUR and no billing address
      it("create-payment-call-test", () => {
        let data = getConnectorDetails("stripe")["pm_list"]["PaymentIntent"];
        let req_data = data["RequestCurrencyEUR"];
        let res_data = data["Response"];

        cy.createPaymentIntentTest(
          create_payment_body_with_currency("EUR"),
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list which should only have ideal with stripe
      it("payment-method-list-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "pm_list"
        ]["PmListResponse"]["PmListWithStripeForIdeal"];
        cy.paymentMethodListTestLessThanEqualToOnePaymentMethod(
          data,
          globalState
        );
      });
    }
  );

  // Testing for scenario:
  // MCA1 -> Stripe configured with ideal = { country = "NL", currency = "EUR" }
  // MCA2 -> Cybersource configured with credit = { currency = "USD" }
  // Payment is done with currency as INR and no billing address
  // The resultant Payment Method list shouldn't have any payment method
  context(
    `
    MCA1 -> Stripe configured with ideal = { country = "NL", currency = "EUR" }\n
    MCA2 -> Cybersource configured with credit = { currency = "USD" }\n
    Payment is done with currency as INR and no billing address\n
    The resultant Payment Method list shouldn't have any payment method\n
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
        cy.merchantCreateCallTest(merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(apiKeyCreateBody, globalState);
      });

      // stripe connector create with ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          createConnectorBody,
          bank_redirect_ideal_enabled,
          globalState,
          "stripe"
        );
      });

      // cybersource connector create with card credit enabled in USD
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          createConnectorBody,
          card_credit_enabled_in_USD,
          globalState,
          "cybersource"
        );
      });

      // creating payment with currency as INR and no billing address
      it("create-payment-call-test", () => {
        let data = getConnectorDetails("stripe")["pm_list"]["PaymentIntent"];
        let req_data = data["RequestCurrencyINR"];
        let res_data = data["Response"];

        cy.createPaymentIntentTest(
          create_payment_body_with_currency("INR"),
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list which should only have ideal with stripe
      it("payment-method-list-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "pm_list"
        ]["PmListResponse"]["PmListNull"];
        cy.paymentMethodListTestLessThanEqualToOnePaymentMethod(
          data,
          globalState
        );
      });
    }
  );

  // Testing for scenario:
  // MCA1 -> Stripe configured with credit = { country = "US" }
  // MCA2 -> Cybersource configured with credit = { country = "US" }
  // Payment is done with country as US and currency as USD
  // The resultant Payment Method list should have both Stripe and cybersource
  context(
    `
   MCA1 -> Stripe configured with credit = { country = "US" }\n
   MCA2 -> Cybersource configured with credit = { country = "US" }\n
   Payment is done with country as US and currency as USD\n
   The resultant Payment Method list should have both Stripe and Cybersource\n
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
        cy.merchantCreateCallTest(merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(apiKeyCreateBody, globalState);
      });

      // stripe connector create with credit enabled for US
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          createConnectorBody,
          card_credit_enabled_in_US,
          globalState,
          "stripe"
        );
      });

      // cybersource connector create with card credit enabled in US
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          createConnectorBody,
          card_credit_enabled_in_US,
          globalState,
          "cybersource"
        );
      });

      // creating payment with currency as USD and billing address as US
      it("create-payment-call-test", () => {
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

      // payment method list which should only have credit with Stripe and Cybersource
      it("payment-method-list-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "pm_list"
        ]["PmListResponse"]["PmListWithCreditTwoConnector"];
        cy.paymentMethodListTestTwoConnectorsForOnePaymentMethodCredit(
          data,
          globalState
        );
      });
    }
  );

  // Testing for scenario:
  // MCA1 -> Stripe configured with ideal = { country = "NL", currency = "EUR" }
  // MCA2 -> Cybersource configured with ideal = { country = "NL", currency = "EUR" }
  // Payment is done with country as US and currency as EUR
  // The resultant Payment Method list shouldn't have anything
  context(
    `
    MCA1 -> Stripe configured with ideal = { country = "NL", currency = "EUR" }\n
    MCA2 -> Cybersource configured with ideal = { country = "NL", currency = "EUR" }\n
    Payment is done with country as US and currency as EUR\n
    The resultant Payment Method list shouldn't have anything\n
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
        cy.merchantCreateCallTest(merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(apiKeyCreateBody, globalState);
      });

      // stripe connector create with ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          createConnectorBody,
          bank_redirect_ideal_enabled,
          globalState,
          "stripe"
        );
      });

      // cybersource connector create with ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          createConnectorBody,
          bank_redirect_ideal_enabled,
          globalState,
          "cybersource"
        );
      });

      // creating payment with currency as EUR and billing address as US
      it("create-payment-call-test", () => {
        let data = getConnectorDetails("stripe")["pm_list"]["PaymentIntent"];
        let req_data = data["RequestCurrencyEUR"];
        let res_data = data["Response"];

        cy.createPaymentIntentTest(
          create_payment_body_with_currency_country("EUR", "US"),
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list which shouldn't have anything
      it("payment-method-list-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "pm_list"
        ]["PmListResponse"]["PmListNull"];
        cy.paymentMethodListTestLessThanEqualToOnePaymentMethod(
          data,
          globalState
        );
      });
    }
  );

  // Testing for scenario:
  // MCA1 -> Stripe configured with card credit no configs present
  // MCA1 -> Cybersource configured with card credit = { currency = "USD" }
  // and ideal (default config present as = { country = "NL", currency = "EUR" } )
  // Payment is done with country as IN and currency as USD
  // The resultant Payment Method list should have
  // Stripe and cybersource both for credit and none for ideal
  context(
    `
    MCA1 -> Stripe configured with card credit no configs present\n
    MCA2 -> Cybersource configured with card credit = { currency = "USD" }\n
    and ideal (default config present as = { country = "NL", currency = "EUR" })\n
    Payment is done with country as IN and currency as USD\n
    The resultant Payment Method list should have\n
    Stripe and Cybersource both for credit and none for ideal\n
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
        cy.merchantCreateCallTest(merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(apiKeyCreateBody, globalState);
      });

      // stripe connector create with card credit enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          createConnectorBody,
          card_credit_enabled,
          globalState,
          "stripe"
        );
      });

      // cybersource connector create with card credit and ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          createConnectorBody,
          bank_redirect_ideal_and_credit_enabled,
          globalState,
          "cybersource"
        );
      });

      // creating payment with currency as USD and billing address as IN
      it("create-payment-call-test", () => {
        let data = getConnectorDetails("stripe")["pm_list"]["PaymentIntent"];
        let req_data = data["RequestCurrencyUSD"];
        let res_data = data["Response"];

        cy.createPaymentIntentTest(
          create_payment_body_with_currency_country("USD", "IN"),
          req_data,
          res_data,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list which should have credit with stripe and cybersource and no ideal
      it("payment-method-list-call-test", () => {
        let data = getConnectorDetails(globalState.get("connectorId"))[
          "pm_list"
        ]["PmListResponse"]["PmListWithCreditTwoConnector"];
        cy.paymentMethodListTestTwoConnectorsForOnePaymentMethodCredit(
          data,
          globalState
        );
      });
    }
  );
});
