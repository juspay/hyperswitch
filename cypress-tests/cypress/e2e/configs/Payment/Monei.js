import { customerAcceptance } from "./Commons";

// Define successful test cards based on Monei documentation
const successfulNo3DSCardDetails = {
  card_number: "378282246310005", // American Express test card
  card_exp_month: "10",
  card_exp_year: "50",
  card_holder_name: "Test User",
  card_cvc: "737",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4000002500003155", // Visa 3DS test card
  card_exp_month: "10",
  card_exp_year: "50",
  card_holder_name: "Test User",
  card_cvc: "737",
};

const failedNo3DSCardDetails = {
  card_number: "4000000000000002", // Declined card
  card_exp_month: "01",
  card_exp_year: "35",
  card_holder_name: "Test User",
  card_cvc: "123",
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "EUR",
    },
  },
};

// Not used in current tests but defined for future implementation
// const multiUseMandateData = {
//   customer_acceptance: customerAcceptance,
//   mandate_type: {
//     multi_use: {
//       amount: 8000,
//       currency: "EUR",
//     },
//   },
// };

const payment_method_data_3ds = {
  card: {
    last4: "3155",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "INTL HDQTRS-CENTER OWNED",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "400000",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "50",
    card_holder_name: "Test User",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

const payment_method_data_no3ds = {
  card: {
    last4: "0005",
    card_type: "CREDIT",
    card_network: "AmericanExpress",
    card_issuer: "AmericanExpress",
    card_issuing_country: "INDIA",
    card_isin: "378282",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "50",
    card_holder_name: "Test User",
    payment_checks: {
      cvc_check: "pass",
      address_line1_check: "pass",
      address_postal_code_check: "pass",
    },
    authentication_data: null,
  },
  billing: null,
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "EUR",
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
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "on_session",
          payment_method_data: payment_method_data_3ds,
        },
      },
    },
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "on_session",
          payment_method_data: payment_method_data_3ds,
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method: "card",
          attempt_count: 1,
          payment_method_data: payment_method_data_no3ds,
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          attempt_count: 1,
          payment_method_data: payment_method_data_no3ds,
        },
      },
    },
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: failedNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "card_declined",
          error_message: "Your card was declined",
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
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
    Refund: {
      Request: {
        amount: 6000,
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
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
  },
};
