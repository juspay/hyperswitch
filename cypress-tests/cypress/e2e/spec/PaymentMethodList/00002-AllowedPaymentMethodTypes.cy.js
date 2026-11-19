import * as fixtures from "../../../fixtures/imports";
import State from "../../../utils/State";
import {
  bankRedirectIdealAndCreditEnabled,
  cardCreditEnabled,
  createPaymentBodyWithCurrency,
} from "../../configs/PaymentMethodList/Commons";
import getConnectorDetails from "../../configs/PaymentMethodList/Utils";

let globalState;

describe("Payment Method list with allowed_payment_method_types flow tests", () => {
  context(
    `
    MCA1 -> Stripe configured with credit (no currency config) and ideal\n
    MCA2 -> Cybersource configured with credit = { currency = "USD" }\n
    Payment is done with currency as EUR and allowed_payment_method_types = ["ideal"]\n
    The resultant Payment Method list should only have ideal\n
    (stripe credit excluded by allowed_payment_method_types, cybersource credit by currency)\n
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

      // stripe connector create with credit and ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          bankRedirectIdealAndCreditEnabled,
          globalState,
          "stripe",
          "stripe_US_default"
        );
      });

      // cybersource connector create with credit enabled
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

      // creating payment with currency as EUR and allowed_payment_method_types = ["ideal"]
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: {
            currency: "EUR",
            customer_acceptance: null,
            setup_future_usage: "off_session",
            authentication_type: "no_three_ds",
            allowed_payment_method_types: ["ideal"],
          },
          Response: data.ResponseAllowedPaymentMethodTypesIdeal,
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrency("EUR"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list should only have ideal and not credit
      it("payment-method-list-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PmListResponse"][
            "PmListWithIdealOneConnector"
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
    MCA1 -> Stripe configured with credit and ideal\n
    MCA2 -> Cybersource configured with credit\n
    Payment is done with currency as USD and allowed_payment_method_types = ["credit"]\n
    The resultant Payment Method list should only have credit
    with both Stripe and Cybersource (ideal filtered out)\n
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

      // stripe connector create with credit and ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          bankRedirectIdealAndCreditEnabled,
          globalState,
          "stripe",
          "stripe_US_default"
        );
      });

      // cybersource connector create with credit enabled
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

      // creating payment with currency as USD and allowed_payment_method_types = ["credit"]
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: {
            currency: "USD",
            customer_acceptance: null,
            setup_future_usage: "off_session",
            authentication_type: "no_three_ds",
            allowed_payment_method_types: ["credit"],
          },
          Response: data.ResponseAllowedPaymentMethodTypesCredit,
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrency("USD"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list should have credit with stripe and cybersource and no ideal
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
    MCA1 -> Cybersource configured with credit\n
    Payment is done with currency as USD and allowed_payment_method_types = ["ideal"]\n
    ideal is not configured on any connector account\n
    The payment create call should fail with IncorrectPaymentMethodConfiguration (IR_39)\n
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

      // cybersource connector create with credit enabled
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

      // creating payment with allowed_payment_method_types not configured on any connector
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: {
            currency: "USD",
            customer_acceptance: null,
            setup_future_usage: "off_session",
            authentication_type: "no_three_ds",
            allowed_payment_method_types: ["ideal"],
          },
          Response: data.ResponseIncorrectPaymentMethodConfiguration,
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrency("USD"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });
    }
  );

  context(
    `
    MCA1 -> Stripe configured with credit and ideal\n
    MCA2 -> Cybersource configured with credit\n
    Payment is done with currency as USD and allowed_payment_method_types = ["credit", "paypal"]\n
    paypal is not configured on any connector account, but credit is\n
    The payment create call should succeed since atleast one allowed type is configured
    and the resultant Payment Method list should only have credit\n
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

      // stripe connector create with credit and ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          bankRedirectIdealAndCreditEnabled,
          globalState,
          "stripe",
          "stripe_US_default"
        );
      });

      // cybersource connector create with credit enabled
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

      // creating payment with currency as USD and allowed_payment_method_types = ["credit", "paypal"]
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: {
            currency: "USD",
            customer_acceptance: null,
            setup_future_usage: "off_session",
            authentication_type: "no_three_ds",
            allowed_payment_method_types: ["credit", "paypal"],
          },
          Response: data.ResponseAllowedPaymentMethodTypesCreditAndPaypal,
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrency("USD"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list should only have credit with stripe and cybersource (paypal contributes nothing)
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
    MCA1 -> Stripe configured with credit = { currency = "USD" }\n
    MCA2 -> Cybersource configured with credit = { currency = "USD" }\n
    Payment is done with currency as EUR and allowed_payment_method_types = ["credit"]\n
    credit is configured on the connectors so create succeeds, but it is not eligible for EUR\n
    The resultant Payment Method list should be empty\n
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
          cardCreditEnabled,
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
          cardCreditEnabled,
          globalState,
          "cybersource",
          "cybersource_US_default"
        );
      });

      // creating payment with currency as EUR and allowed_payment_method_types = ["credit"]
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: {
            currency: "EUR",
            customer_acceptance: null,
            setup_future_usage: "off_session",
            authentication_type: "no_three_ds",
            allowed_payment_method_types: ["credit"],
          },
          Response: data.ResponseAllowedPaymentMethodTypesCredit,
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrency("EUR"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list should be empty since credit is not eligible for EUR
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
    MCA1 -> Adyen configured with credit and ideal\n
    MCA2 -> Cybersource configured with credit = { currency = "USD" }\n
    Payment is done with currency as EUR and allowed_payment_method_types = ["ideal"]\n
    The resultant Payment Method list should only have ideal (via adyen)\n
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

      // adyen connector create with credit and ideal enabled
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          bankRedirectIdealAndCreditEnabled,
          globalState,
          "adyen",
          "adyen_US_default"
        );
      });

      // cybersource connector create with credit enabled
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

      // creating payment with currency as EUR and allowed_payment_method_types = ["ideal"]
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: {
            currency: "EUR",
            customer_acceptance: null,
            setup_future_usage: "off_session",
            authentication_type: "no_three_ds",
            allowed_payment_method_types: ["ideal"],
          },
          Response: data.ResponseAllowedPaymentMethodTypesIdeal,
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrency("EUR"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list should only have ideal (via adyen)
      it("payment-method-list-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PmListResponse"][
            "PmListWithIdealOneConnector"
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
    MCA1 -> Adyen configured with credit = { currency = "USD" }\n
    MCA2 -> Authorizedotnet configured with credit = { currency = "USD" }\n
    Payment is done with currency as USD and allowed_payment_method_types = ["credit"]\n
    The resultant Payment Method list should only have credit
    with both Adyen and Authorizedotnet\n
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

      // adyen connector create with card credit enabled in USD
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditEnabled,
          globalState,
          "adyen",
          "adyen_US_default"
        );
      });

      // authorizedotnet connector create with card credit enabled in USD
      it("connector-create-call-test", () => {
        cy.createNamedConnectorCallTest(
          "payment_processor",
          fixtures.createConnectorBody,
          cardCreditEnabled,
          globalState,
          "authorizedotnet",
          "authorizedotnet_default"
        );
      });

      // creating payment with currency as USD and allowed_payment_method_types = ["credit"]
      it("create-payment-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PaymentIntent"];

        const newData = {
          ...data,
          Request: {
            currency: "USD",
            customer_acceptance: null,
            setup_future_usage: "off_session",
            authentication_type: "no_three_ds",
            allowed_payment_method_types: ["credit"],
          },
          Response: data.ResponseAllowedPaymentMethodTypesCredit,
        };

        cy.createPaymentIntentTest(
          createPaymentBodyWithCurrency("USD"),
          newData,
          "no_three_ds",
          "automatic",
          globalState
        );
      });

      // payment method list should have credit with adyen and authorizedotnet
      it("payment-method-list-call-test", () => {
        const data =
          getConnectorDetails("connector")["pm_list"]["PmListResponse"][
            "PmListWithCreditAdyenAuthorizedotnet"
          ];
        cy.paymentMethodListTestTwoConnectorsForOnePaymentMethodCredit(
          data,
          globalState
        );
      });
    }
  );
});
