const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON:
          "HIGH severity bug: Card payments fail with HE_00 error - payments stuck in processing state. Likely invalid connector credentials or platform bug.",
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
        SKIP_REASON: "HIGH severity bug: Card payments fail with HE_00 error",
      },
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
        },
      },
    },
    No3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "HIGH severity bug: Card payments fail with HE_00 error",
      },
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
        },
      },
    },
    Capture: {
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON:
          "HIGH severity bug: Card payments fail with HE_00 error - capture cannot succeed without successful payment",
      },
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
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "HIGH severity bug: Card payments fail with HE_00 error",
      },
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
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON:
          "HIGH severity bug: Card payments fail with HE_00 error - refund cannot succeed without successful payment",
      },
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
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "HIGH severity bug: Card payments fail with HE_00 error",
      },
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
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "HIGH severity bug: Card payments fail with HE_00 error",
      },
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
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "HIGH severity bug: Card payments fail with HE_00 error",
      },
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
      Configs: {
        TRIGGER_SKIP: true,
        SKIP_REASON: "HIGH severity bug: Card payments fail with HE_00 error",
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
  },
};
