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
      status: 500,
      body: {
        error: {
          type: "server_not_available",
          code: "HE_00",
          message: "Something went wrong",
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
      status: 500,
      body: {
        error: {
          type: "server_not_available",
          code: "HE_00",
          message: "Something went wrong",
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
      status: 500,
      body: {
        error: {
          type: "server_not_available",
          code: "HE_00",
          message: "Something went wrong",
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
      status: 500,
      body: {
        error: {
          type: "server_not_available",
          code: "HE_00",
          message: "Something went wrong",
        },
      },
    },
  },
};
