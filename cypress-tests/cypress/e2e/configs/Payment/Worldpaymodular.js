// WorldpayModular connector configuration
// NOTE: This connector supports wallets only (Apple Pay, Google Pay, Mandates)
// Card payments are NOT supported - API returns IR_19 "card is not supported by worldpaymodular"

import { getCustomExchange } from "./Modifiers";

const customerAcceptance = {
  acceptance_type: "offline",
  accepted_at: "1963-05-03T04:07:52.723Z",
  online: {
    ip_address: "125.0.0.1",
    user_agent: "amet irure esse",
  },
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "Capability mismatch: worldpaymodular does not support card payments. API returns IR_19: 'card is not supported by worldpaymodular'. Connector only supports wallet payments (Apple Pay, Google Pay, Mandates).",
      },
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
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "Capability mismatch: worldpaymodular does not support card payments",
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "08",
            card_exp_year: "30",
            card_holder_name: "joseph Doe",
            card_cvc: "999",
          },
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "card is not supported by worldpaymodular",
            code: "IR_19",
          },
        },
      },
    },
    No3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "Capability mismatch: worldpaymodular does not support card payments",
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "08",
            card_exp_year: "30",
            card_holder_name: "joseph Doe",
            card_cvc: "999",
          },
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "card is not supported by worldpaymodular",
            code: "IR_19",
          },
        },
      },
    },
    Capture: {
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "Capability mismatch: worldpaymodular does not support card payments",
      },
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "card is not supported by worldpaymodular",
            code: "IR_19",
          },
        },
      },
    },
    PartialCapture: {
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "Capability mismatch: worldpaymodular does not support card payments",
      },
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "card is not supported by worldpaymodular",
            code: "IR_19",
          },
        },
      },
    },
    Refund: {
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "Capability mismatch: worldpaymodular does not support card payments",
      },
      Request: {
        amount: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "card is not supported by worldpaymodular",
            code: "IR_19",
          },
        },
      },
    },
    PartialRefund: {
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "Capability mismatch: worldpaymodular does not support card payments",
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "card is not supported by worldpaymodular",
            code: "IR_19",
          },
        },
      },
    },
    manualPaymentRefund: {
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "Capability mismatch: worldpaymodular does not support card payments",
      },
      Request: {
        amount: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "card is not supported by worldpaymodular",
            code: "IR_19",
          },
        },
      },
    },
    manualPaymentPartialRefund: {
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "Capability mismatch: worldpaymodular does not support card payments",
      },
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "card is not supported by worldpaymodular",
            code: "IR_19",
          },
        },
      },
    },
    SyncRefund: {
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "Capability mismatch: worldpaymodular does not support card payments",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "card is not supported by worldpaymodular",
            code: "IR_19",
          },
        },
      },
    },
  },

  // Wallet payment methods are supported - placeholder for future wallet test implementation
  wallet_pm: {
    PaymentIntent: {
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "Wallet payment test implementation pending. Connector supports Apple Pay, Google Pay, and Mandates but automated test configs need to be developed.",
      },
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
  },
};
