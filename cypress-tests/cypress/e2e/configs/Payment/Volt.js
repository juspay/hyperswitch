import { standardBillingAddress } from "./Commons";

const zeroAuthResponse = {
  status: 501,
  body: {
    error: {
      message: "Setup Mandate flow for Volt is not implemented",
      code: "IR_00",
      type: "invalid_request",
    },
  },
};

const zeroAuthConfigs = {
  TRIGGER_SKIP: true,
};

const zeroAuthConfirmPayment = {
  Request: {
    payment_type: "setup_mandate",
    payment_method: "card",
    payment_method_data: {
      card: {
        card_number: "4242424242424242",
        card_exp_month: "01",
        card_exp_year: "50",
        card_holder_name: "joseph Doe",
        card_cvc: "123",
      },
    },
  },
  Response: zeroAuthResponse,
  Configs: zeroAuthConfigs,
};

const zeroAuthMandate = {
  Configs: zeroAuthConfigs,
  Response: zeroAuthResponse,
};

const listRevokeMandate = {
  Configs: zeroAuthConfigs,
  Response: zeroAuthResponse,
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
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
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    No3DSManualCapture: {
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
        status: 200,
        body: {
          status: "requires_capture",
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
          status: "succeeded",
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
          status: "partially_captured",
        },
      },
    },
    Refund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    PartialRefund: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    manualPaymentRefund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    manualPaymentPartialRefund: {
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "USD",
        payment_type: "setup_mandate",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    ZeroAuthConfirmPayment: zeroAuthConfirmPayment,
    ZeroAuthMandate: zeroAuthMandate,
    ListRevokeMandate: listRevokeMandate,
  },
  bank_redirect_pm: {
    OpenBankingUk: {
      Request: {
        payment_method: "bank_redirect",
        amount: 6000,
        currency: "GBP",
        payment_method_type: "open_banking_uk",
        payment_method_data: {
          bank_redirect: {
            open_banking_uk: {
              issuer: "citi",
              country: "GB",
            },
          },
        },
        billing: standardBillingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_type: "open_banking_uk",
          payment_method_type_display_name: "Open Banking",
          connector: "volt",
        },
      },
    },
  },
};
