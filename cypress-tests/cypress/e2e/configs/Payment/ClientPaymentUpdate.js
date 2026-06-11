// Client Payment Update - Feature-specific test configurations
// These configurations are connector-agnostic and used for error-case testing.

export const PaymentUpdateClientAuthConfigs = {
  // Happy path: Successful update with client auth (when enabled)
  HappyPath: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: {
          card_number: "378282246310005",
          card_exp_month: "10",
          card_exp_year: "50",
          card_holder_name: "morino",
          card_cvc: "737",
        },
      },
    },
    Response: {
      status: 200,
      body: {
        status: "requires_confirmation",
      },
    },
  },

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
          type: "api",
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
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          code: "IR_09",
          message:
            "The client_secret provided does not match the client_secret associated with the Payment",
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
      status: 401,
      body: {
        error: {
          type: "invalid_request",
          code: "IR_18",
          message: "Unauthorised access to update customer",
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
          code: "IR_16",
          message:
            "You cannot update this payment because it has status succeeded",
        },
      },
    },
  },
};
