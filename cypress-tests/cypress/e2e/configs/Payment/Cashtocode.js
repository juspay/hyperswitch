import { customerAcceptance } from "./Commons";

// Payment method data for Reward (empty as per domain model)
const payment_method_data_reward = {
  reward: {},
  billing: null,
};

export const connectorDetails = {
  reward_pm: {
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
        amount: 6000,
        authentication_type: "no_three_ds",
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
    SessionToken: {
      Response: {
        status: 200,
        body: {
          session_token: [],
        },
      },
    },
    PaymentIntentWithShippingCost: {
      Request: {
        currency: "USD",
        shipping_cost: 50,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          shipping_cost: 50,
          amount: 6000,
        },
      },
    },
    PaymentConfirmWithShippingCost: {
      Request: {
        payment_method: "reward",
        payment_method_type: "classic_reward",
        payment_method_data: {
          reward: {},
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          shipping_cost: 50,
          amount_received: 6050,
          amount: 6000,
          net_amount: 6050,
        },
      },
    },
    ClassicRewardAutoCapture: {
      Request: {
        payment_method: "reward",
        payment_method_type: "classic_reward",
        payment_method_data: {
          reward: {},
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "reward",
          attempt_count: 1,
          payment_method_data: payment_method_data_reward,
        },
      },
    },
    ClassicRewardManualCapture: {
      Request: {
        payment_method: "reward",
        payment_method_type: "classic_reward",
        payment_method_data: {
          reward: {},
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "reward",
          attempt_count: 1,
          payment_method_data: payment_method_data_reward,
        },
      },
    },
    EvoucherAutoCapture: {
      Request: {
        payment_method: "reward",
        payment_method_type: "evoucher",
        payment_method_data: {
          reward: {},
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "reward",
          attempt_count: 1,
          payment_method_data: payment_method_data_reward,
        },
      },
    },
    EvoucherManualCapture: {
      Request: {
        payment_method: "reward",
        payment_method_type: "evoucher",
        payment_method_data: {
          reward: {},
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method: "reward",
          attempt_count: 1,
          payment_method_data: payment_method_data_reward,
        },
      },
    },
    ClassicRewardFailPayment: {
      Request: {
        payment_method: "reward",
        payment_method_type: "classic_reward",
        payment_method_data: {
          reward: {},
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
            message: "Invalid authentication credentials",
            code: "IR_14",
          },
        },
      },
    },
    EvoucherFailPayment: {
      Request: {
        payment_method: "reward",
        payment_method_type: "evoucher",
        payment_method_data: {
          reward: {},
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
            message: "Invalid authentication credentials",
            code: "IR_14",
          },
        },
      },
    },
    Capture: {
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
          amount_received: null,
        },
      },
    },
    PartialCapture: {
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6000,
          amount_capturable: 6000,
          amount_received: null,
        },
      },
    },
    Void: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },
    Refund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Refunds are not supported for this payment method",
            code: "IR_14",
          },
        },
      },
    },
    manualPaymentRefund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Refunds are not supported for this payment method",
            code: "IR_14",
          },
        },
      },
    },
    manualPaymentPartialRefund: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Refunds are not supported for this payment method",
            code: "IR_14",
          },
        },
      },
    },
    PartialRefund: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Refunds are not supported for this payment method",
            code: "IR_14",
          },
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Refunds are not supported for this payment method",
            code: "IR_14",
          },
        },
      },
    },
    ZeroAuthMandate: {
      Request: {
        payment_method: "reward",
        payment_method_type: "classic_reward",
        payment_method_data: {
          reward: {},
        },
        currency: "USD",
        mandate_data: {
          customer_acceptance: customerAcceptance,
          mandate_type: {
            single_use: {
              amount: 8000,
              currency: "USD",
            },
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Mandates are not supported for this payment method",
            code: "IR_14",
          },
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "USD",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    ZeroAuthConfirmPayment: {
      Request: {
        payment_type: "setup_mandate",
        payment_method: "reward",
        payment_method_type: "classic_reward",
        payment_method_data: {
          reward: {},
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Mandates are not supported for this payment method",
            code: "IR_14",
          },
        },
      },
    },
    SaveCardUseClassicRewardAutoCapture: {
      Request: {
        payment_method: "reward",
        payment_method_type: "classic_reward",
        payment_method_data: {
          reward: {},
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Card saving is not supported for this payment method",
            code: "IR_14",
          },
        },
      },
    },
    SaveCardUseEvoucherAutoCapture: {
      Request: {
        payment_method: "reward",
        payment_method_type: "evoucher",
        payment_method_data: {
          reward: {},
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Card saving is not supported for this payment method",
            code: "IR_14",
          },
        },
      },
    },
    PaymentMethodIdMandateClassicRewardAutoCapture: {
      Request: {
        payment_method: "reward",
        payment_method_type: "classic_reward",
        payment_method_data: {
          reward: {},
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Mandates are not supported for this payment method",
            code: "IR_14",
          },
        },
      },
    },
    PaymentMethodIdMandateEvoucherAutoCapture: {
      Request: {
        payment_method: "reward",
        payment_method_type: "evoucher",
        payment_method_data: {
          reward: {},
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Mandates are not supported for this payment method",
            code: "IR_14",
          },
        },
      },
    },
  },
  pm_list: {
    PmListResponse: {
      PmListNull: {
        payment_methods: [],
      },
      pmListDynamicFieldWithoutBilling: {
        payment_methods: [
          {
            payment_method: "reward",
            payment_method_types: [
              {
                payment_method_type: "classic_reward",
                required_fields: {},
              },
              {
                payment_method_type: "evoucher",
                required_fields: {},
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithBilling: {
        payment_methods: [
          {
            payment_method: "reward",
            payment_method_types: [
              {
                payment_method_type: "classic_reward",
                required_fields: {},
              },
              {
                payment_method_type: "evoucher",
                required_fields: {},
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithNames: {
        payment_methods: [
          {
            payment_method: "reward",
            payment_method_types: [
              {
                payment_method_type: "classic_reward",
                required_fields: {},
              },
              {
                payment_method_type: "evoucher",
                required_fields: {},
              },
            ],
          },
        ],
      },
      pmListDynamicFieldWithEmail: {
        payment_methods: [
          {
            payment_method: "reward",
            payment_method_types: [
              {
                payment_method_type: "classic_reward",
                required_fields: {},
              },
              {
                payment_method_type: "evoucher",
                required_fields: {},
              },
            ],
          },
        ],
      },
    },
  },
};
