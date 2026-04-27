export const connectorDetails = {
  wallet_pm: {
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
      Request: {
        payment_method: "wallet",
        payment_method_type: "amazon_pay",
        payment_method_data: {
          wallet: {
            amazon_pay: {
              checkout_session_id: "amz-checkout-session-test-001",
            },
          },
        },
        shipping: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "San Fransico",
            state: "California",
            zip: "94122",
            country: "US",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "8056594427",
            country_code: "+91",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    Refund: {
      Request: {
        amount: 6500,
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

export const amazonPayPaymentMethodsEnabled = [
  {
    payment_method: "wallet",
    payment_method_types: [
      {
        payment_method_type: "amazon_pay",
        payment_experience: "redirect_to_url",
        minimum_amount: 1,
        maximum_amount: 68607706,
        recurring_enabled: false,
        installment_payment_enabled: false,
      },
    ],
  },
];
