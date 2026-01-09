export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "CAD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
  },
  bank_redirect_pm: {
    Interac: {
      Request: {
        payment_method: "bank_redirect",
        payment_method_type: "interac",
        payment_method_data: {
          bank_redirect: {
            interac: {},
          },
          billing: {
            email: "guest@example.com",
            address: {
              first_name: "John",
              last_name: "Doe",
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
            country: "CA",
            first_name: "joseph",
            last_name: "Doe",
          },
          phone: {
            number: "9123456789",
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
  },
};
