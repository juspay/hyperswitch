const ach_bank_debit_data = {
  account_number: "123456789",
  routing_number: "110000000",
  bank_account_holder_name: "Test User",
  bank_type: "checking",
  bank_holder_type: "personal",
  billing: {
    address: {
      first_name: "Test",
      last_name: "User",
      line1: "123 Test St",
      city: "San Francisco",
      state: "CA",
      zip: "94122",
      country: "US",
    },
    email: "test@example.com",
  },
};

const billing_data = {
  address: {
    first_name: "Test",
    last_name: "User",
    line1: "123 Test St",
    city: "San Francisco",
    state: "CA",
    zip: "94122",
    country: "US",
  },
  email: "test@example.com",
};

const ir04_error = {
  type: "invalid_request",
  message: "Missing required param: connector_customer_id",
  code: "IR_04",
};

export const connectorDetails = {
  bank_debit_pm: {
    PaymentIntent: (paymentMethodType) => {
      if (paymentMethodType !== "Ach") {
        return {
          Configs: {
            TRIGGER_SKIP: true,
          },
        };
      }
      return {
        Request: {
          currency: "USD",
          setup_future_usage: "off_session",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      };
    },
    Ach: {
      Request: {
        payment_method: "bank_debit",
        payment_method_type: "ach",
        payment_method_data: {
          bank_debit: {
            ach_bank_debit: ach_bank_debit_data,
          },
        },
        billing: billing_data,
      },
      Response: {
        status: 400,
        body: {
          error: ir04_error,
        },
      },
    },
    AchDirectConfirm: {
      Request: {
        currency: "USD",
        payment_method: "bank_debit",
        payment_method_type: "ach",
        payment_method_data: {
          bank_debit: {
            ach_bank_debit: ach_bank_debit_data,
          },
        },
        billing: billing_data,
      },
      Response: {
        status: 400,
        body: {
          error: ir04_error,
        },
      },
    },
    AchWithConnectorCustomerId: {
      Request: {
        currency: "USD",
        payment_method: "bank_debit",
        payment_method_type: "ach",
        payment_method_data: {
          bank_debit: {
            ach_bank_debit: ach_bank_debit_data,
          },
        },
        billing: billing_data,
        connector_customer_id: "f338b9dc-af90-4022-9157-0ab5459ee20a",
      },
      Response: {
        status: 400,
        body: {
          error: {
            error_type: "invalid_request",
            message:
              "Json deserialize error: unknown field `connector_customer_id`",
            code: "IR_06",
          },
        },
      },
    },
  },
};
