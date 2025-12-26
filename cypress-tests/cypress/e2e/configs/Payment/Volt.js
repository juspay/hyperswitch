import { getCustomExchange } from "./Modifiers";

const billingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "California",
    zip: "94122",
    country: "GB",
    first_name: "john",
    last_name: "doe",
  },
};

export const connectorDetails = {
  card_pm: {
    ZeroAuthPaymentIntent: {
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "USD",
        payment_type: "setup_mandate",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    ZeroAuthConfirmPayment: {
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "4242424242424242",
            card_exp_month: "01",
            card_exp_year: "50",
            card_holder_name: "joseph Doe",
            card_cvc: "123",
          },
        },
      },
      Response: {
        status: 501,
        body: {
          error: {
            message: "Setup Mandate flow for Volt is not implemented",
            code: "IR_00",
            type: "invalid_request",
          },
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    },
    ZeroAuthMandate: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Response: {
        status: 501,
        body: {
          error: {
            message: "Setup Mandate flow for Volt is not implemented",
            code: "IR_00",
            type: "invalid_request",
          },
        },
      },
    },
    ListRevokeMandate: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Response: {
        status: 501,
        body: {
          error: {
            message: "Setup Mandate flow for Volt is not implemented",
            code: "IR_00",
            type: "invalid_request",
          },
        },
      },
    },
  },
  bank_redirect_pm: {
    OpenBankingUk: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        amount: 6000,
        currency: "GBP",
        payment_method_type: "open_banking_uk",
        payment_method_data: {
          bank_redirect: {
            open_banking_uk: {
              issuer: "citi",
              country: "GB",
            },
          },
        },
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "open_banking_uk",
          payment_method_type_display_name: "Open Banking",
          connector: "volt",
        },
      },
    }),
    OpenBanking: getCustomExchange({
      Request: {
        payment_method: "bank_redirect",
        amount: 6000,
        currency: "EUR",
        payment_method_type: "open_banking",
        payment_method_data: {
          bank_redirect: {
            open_banking: {},
          },
        },
        billing: {
          ...billingAddress,
          address: {
            ...billingAddress.address,
            country: "DE",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "open_banking",
          payment_method_type_display_name: "Open Banking",
          connector: "volt",
        },
      },
    }),
  },
};
