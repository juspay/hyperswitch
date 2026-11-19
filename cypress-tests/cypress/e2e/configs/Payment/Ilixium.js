import { getCustomExchange } from "./Modifiers";
import {
  connectorDetails as commonsConnectorDetails,
  OFFER_QUOTE_ID_PLACEHOLDER,
} from "./Commons";

const verifiedCardDetails = {
  card_number: "9000100111111117",
  card_exp_month: "06",
  card_exp_year: "28",
  card_holder_name: "John Doe",
  card_cvc: "111",
};

const threeDsCardDetails = {
  card_number: "9001100511111112",
  card_exp_month: "06",
  card_exp_year: "28",
  card_holder_name: "John Doe",
  card_cvc: "111",
};

// Ilixium requires the customer's date of birth (schema-mandatory on
// accounts that enforce it; omitting it gets rejected with `VA8`). The
// connector reads it from the standard top-level `customer.date_of_birth`
// request field, NOT from connector-specific `metadata` -- it previously
// lived under `metadata.ilixium_date_of_birth`, which the connector never
// reads, so every request silently omitted the DOB and got rejected.
const ilixiumCustomer = {
  date_of_birth: "1990-01-01",
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        amount: 1000,
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    No3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        amount: 1000,
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    "3DSManualCapture": getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        amount: 1000,
        payment_method_data: {
          card: threeDsCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        authentication_type: "three_ds",
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    "3DSAutoCapture": getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        amount: 1000,
        payment_method_data: {
          card: threeDsCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        authentication_type: "three_ds",
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    No3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        amount: 1000,
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        customer: ilixiumCustomer,
      },
      // Creds we currently have only supports manual capture. Therefore mapped error code for auto capture.
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    PaymentConfirmWithShippingCost: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        customer_acceptance: null,
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    Capture: getCustomExchange({
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
    }),
    PartialCapture: getCustomExchange({
      Request: {
        amount_to_capture: 500,
      },
      Response: {
        status: 200,
        body: {
          status: "partially_captured",
          amount: 1000,
          amount_capturable: 0,
          amount_received: 500,
        },
      },
    }),
    Void: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "cancelled",
        },
      },
    },
    No3DSFailPayment: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    SaveCardUseNo3DSAutoCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    SaveCardUseNo3DSAutoCaptureOffSession: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        setup_future_usage: "off_session",
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    SaveCardUse3DSAutoCaptureOffSession: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: threeDsCardDetails,
        },
        setup_future_usage: "off_session",
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    SaveCardUseNo3DSManualCapture: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        setup_future_usage: "on_session",
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),
    SaveCardUseNo3DSManualCaptureOffSession: getCustomExchange({
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        amount: 1000,
        setup_future_usage: "off_session",
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),
    SaveCardConfirmManualCaptureOffSession: getCustomExchange({
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 5000,
        },
      },
      Request: {
        setup_future_usage: "off_session",
      },
      Response: {
        status: 501,
        body: {
          error: {
            message:
              "This feature is not implemented: repeat_payment flow for ilixium is not implemented",
            type: "invalid_request",
            code: "IR_00",
          },
        },
      },
    }),
    manualPaymentRefund: getCustomExchange({
      Request: {
        amount: 500,
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
        amount: 200,
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
    PaymentMethodIdMandateNo3DSAutoCapture: getCustomExchange({
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    PaymentMethodIdMandateNo3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer: ilixiumCustomer,
      },
    }),
    PaymentMethodIdMandate3DSAutoCapture: getCustomExchange({
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: threeDsCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    }),
    PaymentMethodIdMandate3DSManualCapture: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: threeDsCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer: ilixiumCustomer,
      },
    }),
  },
  // ilixium doesn't implement repeat_payment (see
  // SaveCardConfirmManualCaptureOffSession above), so a saved-card confirm
  // can never succeed here -- override just these two offer_engine keys
  // (merged in via getConnectorDetails()) so the Offer Engine suite's
  // saved-card scenario expects that failure instead of the generic
  // success shared by every other connector.
  offer_engine: {
    // Same as Commons.js's default, just with the DOB ilixium requires added
    // onto the Request.
    OfferEligibilityCheck: {
      ...commonsConnectorDetails.offer_engine.OfferEligibilityCheck,
      Request: {
        ...commonsConnectorDetails.offer_engine.OfferEligibilityCheck.Request,
        customer: ilixiumCustomer,
      },
    },
    // Same pre-existing limitation as card_pm.No3DSAutoCapture above: our
    // creds only support manual capture, so any auto-capture confirm --
    // offer-engine or not -- comes back 200 with this mapped error instead
    // of actually succeeding.
    ConfirmWithOfferApplied: {
      ...commonsConnectorDetails.offer_engine.ConfirmWithOfferApplied,
      Request: {
        ...commonsConnectorDetails.offer_engine.ConfirmWithOfferApplied.Request,
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    },
    AppliedOfferOnRetrieve: getCustomExchange({
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "failed",
        },
      },
    }),
    ConfirmWithoutOffer: {
      ...commonsConnectorDetails.offer_engine.ConfirmWithoutOffer,
      Request: {
        ...commonsConnectorDetails.offer_engine.ConfirmWithoutOffer.Request,
        customer: ilixiumCustomer,
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_message: "4",
          error_code: "4",
        },
      },
    },
    // The "save a card" step earlier in this scenario (card_pm.
    // SaveCardUseNo3DSAutoCapture) itself fails for ilixium (pre-existing
    // auto-capture limitation, see that fixture above) -- no payment_token
    // ever gets saved, so this confirm reaches hyperswitch core with no
    // saved payment method and no raw card data, and core itself (not the
    // connector) rejects it before ever reaching ilixium.
    ConfirmWithOfferAppliedSavedCard: getCustomExchange({
      Request: {
        offer_details: {
          offer_quote_ids: [OFFER_QUOTE_ID_PLACEHOLDER],
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            message: "Missing required param: payment_method_data",
          },
        },
      },
    }),
    AppliedOfferOnRetrieveSavedCard: getCustomExchange({
      Configs: {
        skipBillingAssertion: true,
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
  },
};
