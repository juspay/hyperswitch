const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "03",
  card_exp_year: "30",
  card_holder_name: "John Doe",
  card_cvc: "737",
};

const BillingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "CA",
    zip: "94122",
    country: "US",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "9123456789",
    country_code: "+91",
  },
};

export const connectorDetails = {
  reward_pm: {
    PaymentIntentUSD: {
      Request: {
        currency: "USD",
        amount: 6000,
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: BillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentIntentEUR: {
      Request: {
        currency: "EUR",
        amount: 6000,
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: BillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    Evoucher: {
      Request: {
        payment_method: "reward",
        payment_method_type: "evoucher",
        payment_method_data: "reward",
        billing: BillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "reward",
          payment_method_type: "evoucher",
          amount: 6000,
        },
      },
    },
    Classic: {
      Request: {
        payment_method: "reward",
        payment_method_type: "classic",
        payment_method_data: "reward",
        billing: BillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "reward",
          payment_method_type: "classic",
          amount: 6000,
        },
      },
    },
  },
  card_pm: {
    ZeroAuthMandate: {
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Cashtocode is not implemented",
            code: "IR_00",
          },
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "USD",
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
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Cashtocode is not implemented",
            code: "IR_00",
          },
        },
      },
    },
  },
};
