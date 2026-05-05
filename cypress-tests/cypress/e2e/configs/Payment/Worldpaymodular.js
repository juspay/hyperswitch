const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

const paymentMethodData = {
  card: {
    last4: "1111",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: null,
    card_issuing_country: null,
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "08",
    card_exp_year: "30",
    card_holder_name: "joseph Doe",
    payment_checks: null,
    authentication_data: null,
    auth_code: null,
  },
  billing: null,
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    No3DSAutoCapture: {
      Configs: {
        SKIP_BILLING_VALIDATION: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    No3DSManualCapture: {
      Configs: {
        SKIP_BILLING_VALIDATION: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    Capture: {
      Configs: {
        SKIP_BILLING_VALIDATION: true,
      },
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PartialCapture: {
      Configs: {
        SKIP_BILLING_VALIDATION: true,
      },
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
        },
      },
    },
    Refund: {
      Configs: {
        SKIP_BILLING_VALIDATION: true,
      },
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    PartialRefund: {
      Configs: {
        SKIP_BILLING_VALIDATION: true,
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    manualPaymentRefund: {
      Configs: {
        SKIP_BILLING_VALIDATION: true,
      },
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    manualPaymentPartialRefund: {
      Configs: {
        SKIP_BILLING_VALIDATION: true,
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    SyncRefund: {
      Configs: {
        SKIP_BILLING_VALIDATION: true,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
  },
};
