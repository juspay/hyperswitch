export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "on_session",
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "4242424242424242",
            card_exp_month: "12",
            card_exp_year: "2030",
            card_holder_name: "Test User",
            card_cvc: "123",
          },
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          attempt_count: 1,
        },
      },
    },
    Refund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PartialRefund: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
  },
  pay_later_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        amount: 5000,
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "on_session",
          amount: 5000,
        },
      },
    },
    Affirm: {
      Request: {
        payment_method: "pay_later",
        payment_method_type: "affirm",
        payment_experience: "redirect_to_url",
        currency: "USD",
        amount: 5000,
        payment_method_data: {
          pay_later: {
            affirm_redirect: {},
          },
        },
        billing: {
          address: {
            line1: "123 Test Street",
            city: "San Francisco",
            state: "CA",
            zip: "94102",
            country: "US",
          },
          email: "test@example.com",
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "pay_later",
          payment_method_type: "affirm",
          amount: 5000,
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    },
    AffirmCapture: {
      Request: {
        amount_to_capture: 5000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount_received: 5000,
        },
      },
    },
    AffirmRefund: {
      Request: {
        amount: 5000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 5000,
        },
      },
    },
  },
  webhook: {
    TransactionIdConfig: {
      path: "data.object.id",
      type: "string",
    },
    RefundIdConfig: {
      path: "data.object.id",
      type: "string",
    },
  },
};
