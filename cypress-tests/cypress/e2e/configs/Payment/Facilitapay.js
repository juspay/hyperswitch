import { isoTimeTomorrow } from "../../../utils/RequestBodyUtils";
import { getCustomExchange } from "./Modifiers";

const billingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "California",
    zip: "94122",
    country: "US",
    first_name: "joseph",
    last_name: "Doe",
  },
  phone: {
    number: "9123456789",
    country_code: "+91",
  },
};

const cardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
};

export const connectorDetails = {
  card_pm: {
    ZeroAuthPaymentIntent: {
      Request: {
        currency: "BRL",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    PaymentIntent: {
      Request: {
        currency: "BRL",
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
    No3DSAutoCapture: getCustomExchange({
      Configs: {
        ASSERT_BILLING_NOT_NULL: false,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: cardDetails,
        },
        currency: "BRL",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    No3DSManualCapture: getCustomExchange({
      Configs: {
        ASSERT_BILLING_NOT_NULL: false,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: cardDetails,
        },
        currency: "BRL",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),
    Capture: getCustomExchange({
      Request: {
        amount_to_capture: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    PartialCapture: getCustomExchange({
      Request: {
        amount_to_capture: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
        },
      },
    }),
    Refund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    PartialRefund: getCustomExchange({
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    manualPaymentRefund: getCustomExchange({
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    manualPaymentPartialRefund: getCustomExchange({
      Request: {
        amount: 2000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
    SyncRefund: getCustomExchange({
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    }),
  },
  bank_transfer_pm: {
    Pix: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true, // Skip this test as HandleRedirection is not required to complete the payment flow for Pix.
      },
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix",
        payment_method_data: {
          bank_transfer: {
            pix: {
              // since we pass the same cpf number, the connector customer id will be updated instead of new ones being created
              cpf: "86665623580",
              source_bank_account_id: "739d6b0a-e92a-40fd-9f58-6d4cdeb699bb",
              pix_qr_expiry: isoTimeTomorrow(), // 1 day expiration
            },
          },
        },
        billing: {
          ...billingAddress,
          address: {
            ...billingAddress.address,
            country: "BR",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
      ResponseCustom: {
        status: 200,
        body: {
          error_code: "Cancelled",
          error_reason: "Unable to generate Pix QRCode",
        },
      },
    }),
  },
};
