// Client Payment Update - Feature-specific test configurations
// These configurations are connector-agnostic and used for error-case testing.

export const PaymentUpdateClientAuthConfigs = {
  // Error case: Feature disabled (config not set or set to false)
  FeatureDisabled: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: {
          card_number: "4111111111111111",
          card_exp_month: "08",
          card_exp_year: "30",
          card_holder_name: "Joseph Doe",
          card_cvc: "999",
        },
      },
    },
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          code: "IR_01",
          message: "Payment update via client auth is not enabled",
        },
      },
    },
  },

  // Error case: Invalid client secret
  InvalidClientSecret: {
    Request: {
      client_secret: "invalid_secret_12345",
      payment_method: "card",
      payment_method_data: {
        card: {
          card_number: "4111111111111111",
          card_exp_month: "08",
          card_exp_year: "30",
          card_holder_name: "Joseph Doe",
          card_cvc: "999",
        },
      },
    },
    Response: {
      status: 401,
      body: {
        error: {
          type: "invalid_request",
          code: "IR_01",
          message: "Invalid client secret",
        },
      },
    },
  },

  // Error case: Wrong customer ID
  WrongCustomerId: {
    Request: {
      customer_id: "cus_wrong_customer_12345",
      payment_method: "card",
      payment_method_data: {
        card: {
          card_number: "4111111111111111",
          card_exp_month: "08",
          card_exp_year: "30",
          card_holder_name: "Joseph Doe",
          card_cvc: "999",
        },
      },
    },
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          code: "IR_01",
          message: "Customer ID does not match",
        },
      },
    },
  },

  // Error case: Wrong payment status (e.g., already confirmed/succeeded)
  WrongPaymentStatus: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: {
          card_number: "4111111111111111",
          card_exp_month: "08",
          card_exp_year: "30",
          card_holder_name: "Joseph Doe",
          card_cvc: "999",
        },
      },
    },
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          code: "IR_01",
          message: "Payment cannot be updated in current status",
        },
      },
    },
  },
};
