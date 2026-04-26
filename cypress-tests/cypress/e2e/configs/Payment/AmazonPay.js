export const connectorDetails = {
  wallet_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        amount: 6000,
        capture_method: "automatic",
        payment_method_types: ["wallet"],
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentConfirm: {
      Request: {
        payment_method: "wallet",
        payment_method_type: "amazon_pay",
        payment_method_data: {
          wallet: {
            amazon_pay: {
              confirm_otp: true,
              authentication_otp: {
                otp_value: "123456",
              },
            },
          },
        },
        billing: {
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
          email: "test@example.com",
          phone: {
            number: "9123456789",
            country_code: "+91",
          },
        },
        customer_acceptance: {
          acceptance_type: "online",
          accepted_at: "2024-01-01T00:00:00Z",
          online: {
            ip_address: "125.0.0.1",
            user_agent: "Mozilla/5.0",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "wallet",
          payment_method_type: "amazon_pay",
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
    manualPaymentRefund: {
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
    manualPaymentPartialRefund: {
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
  },
};
