// Configuration for Silverflow mock connector
// This uses the mock server running at http://localhost:3010

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "12",
  card_exp_year: "25",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const PaymentIntentBody = {
  Request: {
    currency: "EUR",
    customer_acceptance: {
      acceptance_type: "offline",
      accepted_at: "2024-01-01T00:00:00.000Z",
      online: {
        ip_address: "127.0.0.1",
        user_agent: "Mozilla/5.0",
      },
    },
  },
  Response: {
    status: 200,
    body: {
      status: "requires_payment_method",
    },
  },
};

const ConfirmBody = {
  Request: {
    payment_method_data: {
      card: successfulNo3DSCardDetails,
    },
    payment_method: "card",
  },
  Response: {
    status: 200,
    body: {
      status: "succeeded",
      amount: 1000,
      amount_capturable: 1000,
      amount_received: 1000,
      connector: "silverflow",
    },
  },
};

const CaptureBody = {
  Request: {
    amount_to_capture: 1000,
  },
  Response: {
    status: 200,
    body: {
      status: "succeeded",
      amount: 1000,
      amount_capturable: 0,
      amount_received: 1000,
    },
  },
};

const RefundBody = {
  Request: {
    amount: 500,
  },
  Response: {
    status: 200,
    body: {
      status: "succeeded",
      amount: 500,
      type: "charged",
    },
  },
};

const SyncRefundBody = {
  Response: {
    status: 200,
    body: {
      status: "succeeded",
      amount: 500,
    },
  },
};

const card_pm = {
  PaymentIntent: PaymentIntentBody,
  PaymentIntentCapture: {
    Request: PaymentIntentBody.Request,
    Response: {
      status: 200,
      body: {
        status: "requires_payment_method",
      },
    },
  },
  No3DSManualCapture: {
    Request: {
      ...ConfirmBody.Request,
      capture_method: "manual",
    },
    Response: {
      status: 200,
      body: {
        status: "requires_capture",
        capture_method: "manual",
      },
    },
  },
  No3DSAutoCapture: ConfirmBody,
  ConfirmWithoutSaveCard: ConfirmBody,
  Confirm: {
    Request: ConfirmBody.Request,
    Response: {
      status: 200,
      body: {
        status: "requires_capture",
      },
    },
  },
  Capture: CaptureBody,
  PartialCapture: {
    Request: {
      amount_to_capture: 500,
    },
    Response: {
      status: 200,
      body: {
        status: "partially_captured",
        amount_capturable: 500,
      },
    },
  },
  Refund: RefundBody,
  PartialRefund: {
    Request: {
      amount: 300,
    },
    Response: {
      status: 200,
      body: {
        status: "succeeded",
        amount: 300,
      },
    },
  },
  SyncRefund: SyncRefundBody,
  payment_method_enabled: ["card"],
};

// Export all the required configurations
export default {
  card_pm,
};
