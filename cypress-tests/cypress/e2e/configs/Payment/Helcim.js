import { getCustomExchange } from "./Modifiers";

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "12",
  card_exp_year: "2030",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const paymentMethodDataNo3DSResponse = {
  card: {
    last4: "1111",
    card_type: null,
    card_network: null,
    card_issuer: null,
    card_issuing_country: null,
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "2030",
    card_holder_name: "Test User",
    payment_checks: null,
    authentication_data: null,
    auth_code: null,
  },
  billing: null,
};

const requiredFields = {
  payment_methods: [
    {
      payment_method: "card",
      payment_method_types: [
        {
          payment_method_type: "credit",
          card_networks: [
            {
              eligible_connectors: ["helcim"],
            },
          ],
          required_fields: [
            {
              field_type: "card_number",
              field_name: "card_number",
              display_name: "card_number",
              placeholder_text: "123456789012",
              required: true,
            },
            {
              field_type: "expiry_date",
              field_name: "card_exp_month",
              display_name: "card_exp_month",
              placeholder_text: "MM",
              required: true,
            },
            {
              field_type: "expiry_date",
              field_name: "card_exp_year",
              display_name: "card_exp_year",
              placeholder_text: "YY",
              required: true,
            },
            {
              field_type: "card_cvc",
              field_name: "card_cvc",
              display_name: "card_cvc",
              placeholder_text: "123",
              required: true,
            },
            {
              field_type: "user_card_holder_name",
              field_name: "card_holder_name",
              display_name: "card_holder_name",
              placeholder_text: "test name",
              required: true,
            },
          ],
        },
        {
          payment_method_type: "debit",
          card_networks: [
            {
              eligible_connectors: ["helcim"],
            },
          ],
          required_fields: [
            {
              field_type: "card_number",
              field_name: "card_number",
              display_name: "card_number",
              placeholder_text: "123456789012",
              required: true,
            },
            {
              field_type: "expiry_date",
              field_name: "card_exp_month",
              display_name: "card_exp_month",
              placeholder_text: "MM",
              required: true,
            },
            {
              field_type: "expiry_date",
              field_name: "card_exp_year",
              display_name: "card_exp_year",
              placeholder_text: "YY",
              required: true,
            },
            {
              field_type: "card_cvc",
              field_name: "card_cvc",
              display_name: "card_cvc",
              placeholder_text: "123",
              required: true,
            },
            {
              field_type: "user_card_holder_name",
              field_name: "card_holder_name",
              display_name: "card_holder_name",
              placeholder_text: "test name",
              required: true,
            },
          ],
        },
      ],
    },
  ],
};

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
    PaymentIntentOffSession: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    No3DSAutoCapture: {
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
          payment_method_data: paymentMethodDataNo3DSResponse,
        },
      },
    },
    No3DSManualCapture: {
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
          payment_method_data: paymentMethodDataNo3DSResponse,
        },
      },
    },
    "3DSAutoCapture": getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        currency: "USD",
      },
    }),
    "3DSManualCapture": getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        currency: "USD",
      },
    }),
    CreditCardAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
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
          payment_method_data: paymentMethodDataNo3DSResponse,
        },
      },
    },
    CreditCardManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
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
          payment_method_data: paymentMethodDataNo3DSResponse,
        },
      },
    },
    DebitCardAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
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
          payment_method_data: {
            ...paymentMethodDataNo3DSResponse,
            card: {
              ...paymentMethodDataNo3DSResponse.card,
              card_type: "DEBIT",
            },
          },
        },
      },
    },
    DebitCardManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
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
          payment_method_data: {
            ...paymentMethodDataNo3DSResponse,
            card: {
              ...paymentMethodDataNo3DSResponse.card,
              card_type: "DEBIT",
            },
          },
        },
      },
    },
    Capture: {
      Request: {
        amount_to_capture: 6543,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PartialCapture: {
      Request: {
        amount_to_capture: 3000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
        },
      },
    },
    Refund: {
      Request: {
        amount: 6543,
        reason: "Customer request",
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
        amount: 3000,
        reason: "Partial refund",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SyncRefund: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    manualPaymentRefund: {
      Request: {
        amount: 6543,
        reason: "Manual payment refund",
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
        amount: 3000,
        reason: "Manual partial refund",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    Void: {
      Request: {
        cancellation_reason: "requested_by_customer",
      },
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },
    requiredFields,
  },
  bank_transfer_pm: {},
  wallet_pm: {},
};
