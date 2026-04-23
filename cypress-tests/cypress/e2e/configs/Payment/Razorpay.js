import {
  cardRequiredField,
  connectorDetails as commonConnectorDetails,
  customerAcceptance,
} from "./Commons";

const successfulNo3DSCardDetails = {
  card_number: "5267 3181 8797 5449",
  card_exp_month: "10",
  card_exp_year: "50",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const successfulUpiDetails = {
  upi: {
    vpa: "success@razorpay",
  },
};

const failedUpiDetails = {
  upi: {
    vpa: "failure@razorpay",
  },
};

export const connectorDetails = {
  order_creation_note: {
    description:
      "Razorpay supports optional order creation via /orders API before payment authorization for UPI payments. " +
      "This is NOT a mandatory connector-specific pre-authorization requirement like BNPL providers. " +
      "No internal order creation occurs within the /payments call.",
    applicable_for: ["upi"],
    flow_type: "optional_pre_authorization",
  },

  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "INR",
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
    PaymentConfirm: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
  },

  upi_pm: {
    PaymentIntent: {
      Request: {
        currency: "INR",
        amount: 10000,
        customer_acceptance: null,
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
        payment_method: "upi",
        payment_method_data: successfulUpiDetails,
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
  },

  order_create_flow: {
    description:
      "Optional order creation flow for Razorpay UPI payments. " +
      "When enabled, creates an order before payment authorization.",
    api_endpoint: "/v1/orders",
    request_fields: ["amount", "currency", "receipt"],
    response_fields: ["id", "amount", "currency", "status"],
    is_optional: true,
    applicable_connectors: ["razorpay"],
    applicable_payment_methods: ["upi"],
  },
};

export default connectorDetails;
