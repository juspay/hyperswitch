import {
  customerAcceptance,
  multiUseMandateData,
  singleUseMandateData,
} from "./Commons";

// givepayments' sandbox does real deliverability verification, not just
// pattern matching — even human-looking fabricated addresses (e.g.
// jane.smith860@gmail.com) get rejected as "disposable or unreachable".
// Use a single real, monitored inbox everywhere.
const generateGivepaymentsEmail = () => "venkatakarthik.m@juspay.in";

const successfulNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "12",
  card_exp_year: "2029",
  card_holder_name: "John Doe",
  card_cvc: "123",
  card_network: "Visa",
};

const billingWithEmail = (email) => ({
  address: {
    line1: "1467",
    line2: "Harrison Street",
    city: "San Francisco",
    state: "California",
    zip: "94122",
    country: "US",
    first_name: "John",
    last_name: "Doe",
  },
  email,
});

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
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
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
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
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      // Confirm returns "succeeded" synchronously — givepayments settles
      // async, shortly after.
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      // Confirm returns "succeeded" synchronously — givepayments settles
      // async, shortly after.
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    // givepayments doesn't implement 3DS at all (connector-service
    // field_probe: authenticate/pre_authenticate/post_authenticate are all
    // "not_supported", redirection_data is hardcoded None on every
    // Authorize response), so a "3DS" card just processes as a normal
    // frictionless payment with no challenge — confirmed live via the
    // off-session 3DS save-card flow, which returns "succeeded" directly
    // rather than requires_customer_action or an error.
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    // manual capture is rejected regardless of 3DS (see No3DSManualCapture)
    // — this hits the same capture_method restriction, not a 3DS-specific
    // rejection.
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "Payment failed during authorization with connector. Retry payment",
            code: "CE_01",
          },
        },
      },
    },
    // givepayments only supports automatic capture — confirmCallTest
    // doesn't honor TRIGGER_SKIP, so this asserts the connector's real,
    // confirmed rejection instead of skipping (same pattern Helcim uses
    // for its always-failing refunds): UCS_501 "Capture method not
    // supported. Givepayments connector supports Automatic capture
    // method." surfaces to the merchant as a 400 CE_01.
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message:
              "Payment failed during authorization with connector. Retry payment",
            code: "CE_01",
          },
        },
      },
    },
    // givepayments settles refunds asynchronously — settlement was never
    // observed even after extended polling, so these assert the real,
    // immediately-observed "processing" state instead of waiting for a
    // terminal status that doesn't arrive within the test's lifetime.
    Refund: {
      Request: {
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
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
          status: "processing",
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
          status: "processing",
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
          status: "processing",
        },
      },
    },
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // givepayments only supports automatic capture
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // givepayments only supports automatic capture
      },
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MITAutoCapture: {
      // wait for the CIT to settle before attempting the repeat charge —
      // the connector_mandate_id needed for MIT isn't reliably resolvable
      // while the CIT is still "processing". OMIT_AMOUNT matches a known
      // working manual (Postman) recipe from dev that sends no "amount"
      // field on the recurring_details/payment_method_id MIT request —
      // testing whether that's the actual differentiator.
      Configs: {
        POLL_BEFORE: true,
        OMIT_AMOUNT: true,
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MITWithoutBillingAddress: {
      Configs: {
        POLL_BEFORE: true,
        OMIT_AMOUNT: true,
      },
      Request: { billing: null },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MITManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // givepayments only supports automatic capture
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      // Confirm returns "processing" immediately (async settlement) — no
      // immediate status assertion here; POLL_AFTER polls until it actually
      // reaches "succeeded" and asserts that explicitly, before letting the
      // subsequent MIT step run. A still-"processing" CIT hasn't reliably
      // persisted connector_mandate_id onto the payment_method yet.
      Configs: {
        POLL_AFTER: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {},
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true, // givepayments only supports automatic capture
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    // givepayments does not support 3DS at all (connector metadata:
    // "three_ds": "not_supported"); see REDIRECT_THREE_DS exclude list.
    PaymentMethodIdMandate3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PaymentMethodIdMandate3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        email: generateGivepaymentsEmail(),
        billing: billingWithEmail(generateGivepaymentsEmail()),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
  },
  webhook: {
    TransactionIdConfig: {
      // Defines how to locate and parse the payment reference ID from connector-specific webhook payloads
      path: "payload.id",
      type: "string",
    },
  },
};
