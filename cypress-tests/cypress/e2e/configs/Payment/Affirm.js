export const connectorDetails = {
  pay_later_pm: {
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
    AffirmAutoCapture: {
      Request: {
        payment_method: "pay_later",
        payment_method_type: "affirm_redirect",
        payment_method_data: {
          pay_later: {
            affirm: {},
          },
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
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
          email: "something@example.com",
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "pay_later",
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
};
