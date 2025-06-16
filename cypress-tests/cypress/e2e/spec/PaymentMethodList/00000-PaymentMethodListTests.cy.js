import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import {
  bankRedirectIdealAndCreditEnabled,
  bankRedirectIdealEnabled,
  cardCreditEnabled,
  cardCreditEnabledInEur,
  cardCreditEnabledInUs,
  cardCreditEnabledInUsd,
  createPaymentBodyWithCurrency,
  createPaymentBodyWithCurrencyCountry,
} from "../../configs/PaymentMethodList/Commons";
import getConnectorDetails from "../../configs/PaymentMethodList/Utils";

let globalState;
describe("Payment Method list using Constraint Graph flow tests", () => {
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
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });
      it("customer-create-call-test", () => {
        cy.createCustomerCallTest(fixtures.customerCreateBody, globalState);
      });

      // stripe connector create with ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          bankRedirectIdealEnabled,
          globalState,
          "stripe",
          "stripe_US_default"
        );
      });

      // cybersource connector create with card credit enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditEnabled,
          globalState,
          "cybersource",
          "cybersource_US_default"
        );
      });

      // creating payment with currency as EUR and no billing address
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: data.RequestCurrencyEUR,
          RequestCurrencyEUR: undefined, // we do not need this anymore
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrency("EUR"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list which should only have ideal with stripe
      it("payment-method-list-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PmListResponse"][
            "PmListWithStripeForIdeal"
          ];
        cy.paymentMethodListTestLessThanEqualToOnePaymentMethod(
          data,
          globalState
        );
      });
    }
  );

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
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });

      // stripe connector create with ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          bankRedirectIdealEnabled,
          globalState,
          "stripe",
          "stripe_US_default"
        );
      });

      // cybersource connector create with card credit enabled in USD
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditEnabledInUsd,
          globalState,
          "cybersource",
          "cybersource_US_default"
        );
      });

      // creating payment with currency as INR and no billing address
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: data.RequestCurrencyINR,
          RequestCurrencyINR: undefined, // we do not need this anymore
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrency("INR"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list which should only have ideal with stripe
      it("payment-method-list-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PmListResponse"][
            "PmListNull"
          ];
        cy.paymentMethodListTestLessThanEqualToOnePaymentMethod(
          data,
          globalState
        );
      });
    }
  );

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
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });

      // stripe connector create with credit enabled for US
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

      // cybersource connector create with card credit enabled in US
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditEnabledInUs,
          globalState,
          "cybersource",
          "cybersource_US_default"
        );
      });

      // creating payment with currency as USD and billing address as US
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: data.RequestCurrencyUSD,
          RequestCurrencyUSD: undefined, // we do not need this anymore
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrency("USD"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list which should only have credit with Stripe and Cybersource
      it("payment-method-list-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PmListResponse"][
            "PmListWithCreditTwoConnector"
          ];
        cy.paymentMethodListTestTwoConnectorsForOnePaymentMethodCredit(
          data,
          globalState
        );
      });
    }
  );

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
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });

      // stripe connector create with ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          bankRedirectIdealEnabled,
          globalState,
          "stripe",
          "stripe_US_default"
        );
      });

      // cybersource connector create with ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          bankRedirectIdealEnabled,
          globalState,
          "cybersource",
          "cybersource_US_default"
        );
      });

      // creating payment with currency as EUR and billing address as US
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: data.RequestCurrencyEUR,
          RequestCurrencyEUR: undefined, // we do not need this anymore
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrencyCountry("EUR", "US", "US"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list which shouldn't have anything
      it("payment-method-list-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PmListResponse"][
            "PmListNull"
          ];
        cy.paymentMethodListTestLessThanEqualToOnePaymentMethod(
          data,
          globalState
        );
      });
    }
  );

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
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });

      // stripe connector create with card credit enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditEnabled,
          globalState,
          "stripe",
          "stripe_US_default"
        );
      });

      // cybersource connector create with card credit and ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          bankRedirectIdealAndCreditEnabled,
          globalState,
          "cybersource",
          "cybersource_US_default"
        );
      });

      // creating payment with currency as USD and billing address as IN
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: data.RequestCurrencyUSD,
          RequestCurrencyUSD: undefined, // we do not need this anymore
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrencyCountry("USD", "IN", "IN"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list which should have credit with stripe and cybersource and no ideal
      it("payment-method-list-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PmListResponse"][
            "PmListWithCreditTwoConnector"
          ];
        cy.paymentMethodListTestTwoConnectorsForOnePaymentMethodCredit(
          data,
          globalState
        );
      });
    }
  );

  context(
    `
   MCA1 -> Stripe configured with card credit\n
   MCA2 -> Cybersource configured with card credit = { currency = "USD" }\n
   Payment is done with currency as USD and no billing address\n
   The resultant Payment Method list should have both\n
   Stripe and cybersource for credit\n
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

      // stripe connector create with card credit enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditEnabled,
          globalState,
          "stripe",
          "stripe_US_default"
        );
      });

      // cybersource connector create with card credit
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditEnabled,
          globalState,
          "cybersource",
          "cybersource_US_default"
        );
      });

      // creating payment with currency as USD and billing address as IN
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: data.RequestCurrencyUSD,
          RequestCurrencyUSD: undefined, // we do not need this anymore
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrency("USD"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list which should have credit with stripe and cybersource and no ideal
      it("payment-method-list-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PmListResponse"][
            "PmListWithCreditTwoConnector"
          ];
        cy.paymentMethodListTestTwoConnectorsForOnePaymentMethodCredit(
          data,
          globalState
        );
      });
    }
  );

  context(
    `
    MCA1 -> Stripe configured with ideal = { country = "NL", currency = "EUR" }\n
    MCA2 -> Cybersource configured with credit = { currency = "USD" }\n
    Payment is done with currency as EUR and billing country as NL , shipping country as US\n
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
        cy.merchantCreateCallTest(fixtures.merchantCreateBody, globalState);
      });

      it("api-key-create-call-test", () => {
        cy.apiKeyCreateTest(fixtures.apiKeyCreateBody, globalState);
      });

      // stripe connector create with ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          bankRedirectIdealEnabled,
          globalState,
          "stripe",
          "stripe_US_default"
        );
      });

      // cybersource connector create with card credit enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditEnabled,
          globalState,
          "cybersource",
          "cybersource_US_default"
        );
      });

      // creating payment with currency as EUR and no billing address
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: data.RequestCurrencyEUR,
          RequestCurrencyEUR: undefined, // we do not need this anymore
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrencyCountry("EUR", "NL", "US"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list which should only have ideal with stripe
      it("payment-method-list-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PmListResponse"][
            "PmListWithStripeForIdeal"
          ];
        cy.paymentMethodListTestLessThanEqualToOnePaymentMethod(
          data,
          globalState
        );
      });
    }
  );

  context(
    `
    MCA1 -> Stripe configured with credit = { currency = "USD" }\n
    MCA2 -> Novalnet configured with credit = { currency = "EUR" }\n
    Payment is done with currency as as USD and no billing address\n
    The resultant Payment Method list should only have credit with stripe\n
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

      // stripe connector create with card credit enabled in USD
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditEnabledInUsd,
          globalState,
          "stripe",
          "stripe_US_default"
        );
      });

      // novalnet connector create with card credit enabled in EUR
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditEnabledInEur,
          globalState,
          "novalnet",
          "novalnet_DE_default"
        );
      });

      // creating payment with currency as USD and no billing email
      // billing.email is mandatory for novalnet
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];
        const newData = {
          ...data,
          Request: data.RequestCurrencyUSD,
          RequestCurrencyUSD: undefined, // we do not need this anymore
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrency("USD"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list should only have credit with stripe
      it("payment-method-list-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PmListResponse"][
            "PmListWithCreditOneConnector"
          ];
        cy.paymentMethodListTestLessThanEqualToOnePaymentMethod(
          data,
          globalState
        );
      });
    }
  );
  context(
    `
    MCA1 -> Stripe configured with credit = { currency = "USD" }\n
    MCA2 -> Novalnet configured with credit = { currency = "EUR" }\n
    Payment is done with currency as as EUR and billing address for 3ds credit card\n
    The resultant Payment Method list should only have credit with novalnet\n
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

      // stripe connector create with card credit enabled in USD
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditEnabledInUsd,
          globalState,
          "stripe",
          "stripe_US_default"
        );
      });

      // novalnet connector create with card credit enabled in EUR
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditEnabledInEur,
          globalState,
          "novalnet",
          "novalnet_DE_default"
        );
      });

      // creating payment with currency as EUR and billing email
      // billing.email is mandatory for novalnet
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];
        const newData = {
          ...data,
          Request: data.RequestCurrencyEUR,
          RequestCurrencyEUR: undefined, // we do not need this anymore
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrencyCountry("EUR", "IN", "IN"),
          newData,
          "three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list should only have credit with novalnet
      it("payment-method-list-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PmListResponse"][
            "PmListWithCreditOneConnector"
          ];
        cy.paymentMethodListTestLessThanEqualToOnePaymentMethod(
          data,
          globalState
        );
      });
    }
  );
});
