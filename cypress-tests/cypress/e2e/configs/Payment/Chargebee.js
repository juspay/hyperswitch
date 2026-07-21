import {
  cardRequiredField,
  connectorDetails as commonConnectorDetails,
  customerAcceptance,
} from "./Commons";
import { getCustomExchange } from "./Modifiers";

const successfulNo3DSCardDetails = {
  card_number: "378282246310005",
  card_exp_month: "10",
  card_exp_year: "50",
  card_holder_name: "morino",
  card_cvc: "737",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4000002500003155",
  card_exp_month: "10",
  card_exp_year: "50",
  card_holder_name: "morino",
  card_cvc: "737",
};

const externalThreeDSCardDetails = {
  card_number: "4242424242424242",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const failedNo3DSCardDetails = {
  card_number: "4000000000000002",
  card_exp_month: "01",
  card_exp_year: "35",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};

const onlineCustomerAcceptance = {
  ...customerAcceptance,
  acceptance_type: "online",
};

const chargebeeTestPriceId =
  Cypress.env("CHARGEBEE_TEST_ITEM_PRICE_ID") ||
  Cypress.env("CHARGEBEE_TEST_PRICE_ID") ||
  "";

const subscriptionBilling = {
  email: "guest@juspay.in",
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Francisco",
    state: "California",
    zip: "94122",
    country: "US",
    first_name: "joseph",
    last_name: "Doe",
  },
  phone: {
    number: "8056599999",
    country_code: "+1",
  },
};

const payment_method_data_3ds = {
  card: {
    last4: "3155",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "INTL HDQTRS CENTER OWNED",
    card_issuing_country: "UNITED STATES OF AMERICA",
    card_isin: "400000",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "50",
    card_holder_name: "morino",
    payment_checks: null,
    authentication_data: null,
    auth_code: null,
  },
  billing: null,
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: getCustomExchange({
      Request: {
        currency: "USD",
        customer_acceptance: customerAcceptance,
        statement_descriptor: "Chargebee Payment",
        setup_future_usage: "on_session",
        authentication_type: "no_three_ds",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    PaymentIntentMandate: getCustomExchange({
      Request: {
        currency: "USD",
        customer_acceptance: onlineCustomerAcceptance,
        statement_descriptor: "Chargebee Mandate Payment",
        setup_future_usage: "off_session",
        mandate_data: multiUseMandateData,
        authentication_type: "no_three_ds",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    "3DS": getCustomExchange({
      Request: {
        currency: "USD",
        customer_acceptance: customerAcceptance,
        statement_descriptor: "Chargebee 3DS Payment",
        authentication_type: "three_ds",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    "3DSMandate": getCustomExchange({
      Request: {
        currency: "USD",
        customer_acceptance: customerAcceptance,
        statement_descriptor: "Chargebee 3DS Mandate",
        setup_future_usage: "off_session",
        mandate_data: multiUseMandateData,
        authentication_type: "three_ds",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    Recurring: getCustomExchange({
      Request: {
        currency: "USD",
        off_session: true,
        statement_descriptor: "Chargebee Recurring Payment",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    RecurringMandate: getCustomExchange({
      Request: {
        currency: "USD",
        mandate_id: commonConnectorDetails.mandate_id,
        off_session: true,
        statement_descriptor: "Chargebee Recurring Mandate",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
  },
  subscription_pm: {
    Create: getCustomExchange({
      Configs: {
        USE_SUBSCRIPTION_CREATE_ENDPOINT: true,
      },
      Request: {
        item_price_id: chargebeeTestPriceId,
        billing: subscriptionBilling,
        payment_details: {
          payment_type: "setup_mandate",
          authentication_type: "no_three_ds",
          setup_future_usage: "off_session",
          capture_method: "automatic",
          return_url: "https://example.com/subscription/return",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "created",
        },
      },
    }),
    CreateInvalidCustomer: getCustomExchange({
      Configs: {
        USE_SUBSCRIPTION_CREATE_ENDPOINT: true,
      },
      Request: {
        customer_id: "cust_invalid_nonexistent",
        item_price_id: chargebeeTestPriceId,
        billing: subscriptionBilling,
        payment_details: {
          payment_type: "setup_mandate",
          authentication_type: "no_three_ds",
          setup_future_usage: "off_session",
          capture_method: "automatic",
          return_url: "https://example.com/subscription/return",
        },
      },
      Response: {
        status: 404,
        body: {
          error: {
            type: "invalid_request",
            code: "HE_02",
            message: "Customer does not exist in our records",
          },
        },
      },
    }),
    CreateMissingFields: getCustomExchange({
      Configs: {
        USE_SUBSCRIPTION_CREATE_ENDPOINT: true,
      },
      Request: {
        skip_dynamic_fields: true,
        description: "Test subscription missing required fields",
      },
      Response: {
        status: 400,
        body: {
          error: {
            error_type: "invalid_request",
            code: "IR_06",
            message: "Json deserialize error: missing field `item_price_id`",
          },
        },
      },
    }),
    Retrieve: getCustomExchange({
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "created",
        },
      },
    }),
    RetrieveCancelled: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    }),
    Update: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        plan_id: chargebeeTestPriceId,
        item_price_id: chargebeeTestPriceId,
      },
      Response: {
        status: 200,
        body: {
          status: "active",
        },
      },
    }),
    Cancel: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        cancel_reason_code: "requested_by_customer",
      },
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    }),
    Resume: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "active",
        },
      },
    }),
  },
};
