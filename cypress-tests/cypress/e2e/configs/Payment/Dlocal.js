import {
  customerAcceptance,
  singleUseMandateData,
  multiUseMandateData,
} from "./Commons";
const mockBillingDetails = {
  address: {
    line1: "Servidao B-1",
    line2: null,
    line3: null,
    city: "Volta Redonda",
    state: "Rio de Janeiro",
    zip: "27275-595",
    country: "BR",
    first_name: "Thiago",
    last_name: "Gabriel",
  },
  phone: {
    number: "123456712345",
    country_code: "+55",
  },
  email: "thiago@example.com",
};
const successfulCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "10",
  card_exp_year: "40",
  card_holder_name: "Thiago Gabriel",
  card_cvc: "123",
};
const failedCardDetails = {
  card_number: "4000000000000002",
  card_exp_month: "10",
  card_exp_year: "40",
  card_holder_name: "Thiago Gabriel",
  card_cvc: "123",
};
const payment_method_data_no3ds = {
  card: {
    last4: "1111",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "JP Morgan",
    card_issuing_country: "INDIA",
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "40",
    card_holder_name: "Thiago Gabriel",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};
const payment_method_data_no3ds_address = {
  card: {
    last4: "1111",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "JP Morgan",
    card_issuing_country: "INDIA",
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "40",
    card_holder_name: "Thiago Gabriel",
    payment_checks: null,
    authentication_data: null,
  },
  billing: mockBillingDetails,
};
const payment_method_data_3ds_address = {
  card: {
    last4: "1111",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "JP Morgan",
    card_issuing_country: "INDIA",
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "10",
    card_exp_year: "40",
    card_holder_name: "Thiago Gabriel",
    payment_checks: null,
    authentication_data: null,
  },
  billing: mockBillingDetails,
};
export const connectorDetails = {
  card_pm: {
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: failedCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "104",
          error_message: "Card declined",
          unified_code: "UE_9000",
          unified_message: "Something went wrong",
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
        payment_method: "card",
        payment_method_data: {
          card: successfulCardDetails,
          billing: mockBillingDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          shipping_cost: 50,
          amount: 6000,
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulCardDetails,
          billing: mockBillingDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_no3ds_address,
        },
      },
    },
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulCardDetails, // Uses updated card details
          billing: mockBillingDetails, // Uses updated billing details
        },
        currency: "BRL",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: mockBillingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method: "card",
          payment_method_data: payment_method_data_no3ds,
        },
      },
    },
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulCardDetails,
          billing: mockBillingDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_data: payment_method_data_3ds_address,
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
      Request: {
        cancellation_reason: "VOID",
      },
      Response: {
        status: 200,
        body: {
          status: "cancelled",
          capture_method: "manual",
        },
      },
    },
    VoidAfterConfirm: {
      Request: {
        cancellation_reason: "VOID",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          capture_method: "manual",
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
    manualPaymentRefund: {
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
    manualPaymentPartialRefund: {
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
    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulCardDetails,
          billing: mockBillingDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        authentication_type: "no_three_ds",
        capture_method: "automatic",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: payment_method_data_no3ds_address,
          payment_method: "card",
          connector: "dlocal",
        },
      },
      Configs: { TRIGGER_SKIP: true },
    },
    MandateSingleUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulCardDetails,
          billing: mockBillingDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
      Configs: { TRIGGER_SKIP: true },
    },
    MITManualCapture: {
      Request: {
        payment_method_data: {
          card: successfulCardDetails,
          billing: mockBillingDetails,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
      Configs: { TRIGGER_SKIP: true },
    },
    MandateSingleUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          payment_method_data: payment_method_data_3ds_address,
          payment_method: "card",
          connector: "dlocal",
        },
      },
      Configs: { TRIGGER_SKIP: true },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulCardDetails,
          billing: mockBillingDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: payment_method_data_no3ds_address,
          payment_method: "card",
          connector: "dlocal",
        },
      },
      Configs: { TRIGGER_SKIP: true },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulCardDetails,
          billing: mockBillingDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: payment_method_data_no3ds_address,
          payment_method: "card",
          connector: "dlocal",
        },
      },
      Configs: { TRIGGER_SKIP: true },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulCardDetails,
          billing: mockBillingDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: payment_method_data_no3ds_address,
          payment_method: "card",
          connector: "dlocal",
        },
      },
      Configs: { TRIGGER_SKIP: true },
    },
    MandateMultiUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulCardDetails,
          billing: mockBillingDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method_data: payment_method_data_no3ds_address,
          payment_method: "card",
          connector: "dlocal",
        },
      },
      Configs: { TRIGGER_SKIP: true },
    },
    ZeroAuthMandate: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulCardDetails,
          billing: mockBillingDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Dlocal is not implemented",
            code: "IR_00",
          },
        },
      },
      Configs: { TRIGGER_SKIP: true },
    },
    ZeroAuthPaymentIntent: {
      Request: {
        currency: "USD",
        amount: 0,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
      Configs: { TRIGGER_SKIP: true },
    },
    ZeroAuthConfirmPayment: {
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_data: {
          card: successfulCardDetails,
          billing: mockBillingDetails,
        },
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Dlocal is not implemented",
            code: "IR_00",
          },
        },
      },
      Configs: { TRIGGER_SKIP: true },
    },
    SyncPayment: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_no3ds_address,
        },
      },
    },
  },
};
