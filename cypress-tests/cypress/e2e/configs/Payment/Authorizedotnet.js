const successfulTestCardDetailsList = [
  {
    card_number: "4622943127013705",
    card_exp_month: "10",
    card_exp_year: "30",
    card_holder_name: "Juspay Hyperswitch",
    card_cvc: "838",
  },
  {
    card_number: "4622943127013713",
    card_exp_month: "10",
    card_exp_year: "30",
    card_holder_name: "Juspay Hyperswitch",
    card_cvc: "043",
  },
  {
    card_number: "4622943127013721",
    card_exp_month: "10",
    card_exp_year: "30",
    card_holder_name: "Juspay Hyperswitch",
    card_cvc: "258",
  },
  {
    card_number: "4622943127013739",
    card_exp_month: "10",
    card_exp_year: "30",
    card_holder_name: "Juspay Hyperswitch",
    card_cvc: "942",
  },
  {
    card_number: "4622943127013747",
    card_exp_month: "10",
    card_exp_year: "30",
    card_holder_name: "Juspay Hyperswitch",
    card_cvc: "370",
  },
];

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
          amount: 6000,
          shipping_cost: 50,
        },
      },
    },
    PaymentConfirmWithShippingCost: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulTestCardDetailsList[0],
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          shipping_cost: 50,
          amount_received: 6050,
          amount: 6000,
          net_amount: 6050,
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulTestCardDetailsList[0],
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
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulTestCardDetailsList[0],
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
    Capture: {
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000,
          amount_capturable: 0,
          amount_received: 6000,
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
          amount: 6000,
          amount_capturable: 0,
          amount_received: 2000,
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
    // The `error_code: "54"` and `error_message: "The referenced transaction does not meet the criteria for issuing a credit."` is expected because the transaction status needs to be `SettledSuccessfully` from the Authorize.net's end and `charged` from Hyperswitch's end but according to the latest code, as soon as the payment is successful, the transaction will get the status as `charged` from Hyperswitch's end, but to initiatte a refund, one needs to wait for 4 to 5 days.
    Refund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          error_message:
            "The referenced transaction does not meet the criteria for issuing a credit.",
          error_code: "54",
        },
      },
    },
    PartialRefund: {
      Response: {
        status: 200,
        body: {
          error_message:
            "The referenced transaction does not meet the criteria for issuing a credit.",
          error_code: "54",
        },
      },
    },
    manualPaymentRefund: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulTestCardDetailsList[0],
        },
        currency: "USD",
        customer_acceptance: null,
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
        payment_method: "card",
        payment_method_data: {
          card: successfulTestCardDetailsList[0],
        },
        currency: "USD",
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
};
