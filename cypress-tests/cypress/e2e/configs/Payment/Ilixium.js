import { getCustomExchange } from "./Modifiers";

// Ilixium is UCS-only. Data below is verified against real request/response
// pairs from https://github.com/juspay/hyperswitch/issues/13708, plus live
// cypress runs for No3DSManualCapture.
const verifiedCardDetails = {
  card_number: "9000100111111117",
  card_exp_month: "06",
  card_exp_year: "28",
  card_holder_name: "John Doe",
  card_cvc: "111",
};

// Ilixium requires the cardholder's date of birth as connector-specific metadata.
const ilixiumMetadata = {
  ilixium_date_of_birth: "01011990",
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
    // Configs.DELAY here applies to the "Retrieve Payment after Confirmation"
    // step in 06-NoThreeDSManualCapture.cy.js too (it reuses this same data
    // object), giving Ilixium's authorize state a moment to settle before the
    // force_sync=true PSync call that step makes — see the Capture/Void
    // entries below for the failure this is meant to avoid.
    // payment_method_type: "credit" added after a refund on a captured
    // Ilixium payment failed with IR_04 "Missing required param:
    // payment_method_type" — the payment attempt never had it recorded
    // because it wasn't set here at confirm time. Inferred fix, not yet
    // independently confirmed to make the refund itself succeed.
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
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    // Same DELAY-before-retrieve reasoning as No3DSManualCapture above — this
    // entry is also reused as the "Retrieve Payment after Confirmation" data
    // source in specs that confirm with authentication_type: "three_ds".
    "3DSManualCapture": getCustomExchange({
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
        authentication_type: "three_ds",
        metadata: ilixiumMetadata,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    }),
    // Automatic capture isn't supported for these creds. Usually HTTP 200
    // status "failed" with error_message/error_code both "4" (same as
    // No3DSAutoCapture below); a live run once returned a hard HTTP 500 with
    // error.type "api" instead, so this may be flaky between the two shapes.
    "3DSAutoCapture": getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        amount: 1000,
        payment_method_data: {
          card: verifiedCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        authentication_type: "three_ds",
        metadata: ilixiumMetadata,
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
        metadata: ilixiumMetadata,
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
    // Same automatic-capture quirk as No3DSAutoCapture above — used by
    // 04-NoThreeDSAutoCapture.cy.js's shipping-cost variant.
    PaymentConfirmWithShippingCost: getCustomExchange({
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: verifiedCardDetails,
        },
        customer_acceptance: null,
        metadata: ilixiumMetadata,
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
    // Unverified: no partial-amount example in issue #13708.
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
    // Commons.js's defaults (amount: 6000 / 2000) exceed what these tests
    // actually capture for ilixium (1000 full / 500 partial), which fails
    // live with IR_13 "The refund amount exceeds the amount captured" — not
    // a connector issue, just a mismatched fixture amount. Both keys below
    // are shared across the "fully captured" (1000 available) and
    // "partially captured" (500 available) refund test contexts, so sized
    // to stay valid for the smaller (500) case in both.
    // IR_04 "Missing required param: payment_method_type" on a live refund
    // run turned out NOT to mean the refund body needs this field — /refunds
    // rejects it outright as an unknown field (IR_06, "Json deserialize
    // error: unknown field `payment_method_type`"). It means the payment
    // attempt itself never had payment_method_type recorded, which refund
    // logic reads back — see the confirm entries above, which now set it.
    // Verified live: refund completes synchronously with status "succeeded"
    // (not "pending" — that was Commons.js's default assumption, wrong here).
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
    // Called twice ("2nd Attempt") in the partial-refund tests — 200 + 200
    // stays within the 500 captured in the partial-capture context.
    // Unverified whether this one is also synchronous "succeeded" like
    // manualPaymentRefund above (not independently confirmed) — check this
    // if it fails the same way.
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
  },
};
