const successfulNo3DSCardDetails = {
  card_number: "4242424242424242",
  card_exp_month: "12",
  card_exp_year: "27",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

const successful3DSCardDetails = {
  card_number: "4000000000001091",
  card_exp_month: "12",
  card_exp_year: "27",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

const billingDetails = {
  billing: {
    country: "ID",
    first_name: "joseph",
    last_name: "Doe",
    line1: "123 Main Street",
    line2: "",
    line3: "",
    city: "San Francisco",
    state: "CA",
    zip: "94122",
  },
  email: "mauro.morandi@nexi.it",
  phone: {
    number: "9123456789",
    country_code: "+91",
  },
};
const customerAcceptance = {
  acceptance_type: "offline",
  accepted_at: "1963-05-03T04:07:52.723Z",
  online: {
    ip_address: "127.0.0.1",
    user_agent: "amet irure esse",
  },
};
const paymentMethodData3ds = {
  card: {
    last4: "1091",
    card_type: "CREDIT",
    card_subtype: "VISA TRADITIONAL",
    card_segment_type: "consumer",
    funding_source: "CREDIT",
    card_network: "Visa",
    card_issuer: "INTL HDQTRS CENTER OWNED",
    card_issuing_country: "UNITED STATES OF AMERICA",
    card_isin: "400000",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "27",
    card_holder_name: "joseph Doe",
    payment_checks: null,
    authentication_data: null,
    auth_code: null,
  },
  billing: {
    address: null,
    email: "mauro.morandi@nexi.it",
    phone: {
      number: "9123456789",
      country_code: "+91",
    },
  },
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 6000000,
      currency: "IDR",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 6000000,
      currency: "IDR",
    },
  },
};

export const blockedPaymentErrorBodyForIssuingCountry = {
  status: 200,
  expectBlockedPayment: true,
  body: {
    error: {
      type: "blocked",
      message:
        "Cards issued in your region aren't supported for this transaction, please try a different card",
      code: "HE_03",
      reason: "Blocked",
    },
  },
};

export const blockedPaymentErrorBodyForDebitCard = {
  status: 200,
  expectBlockedPayment: true,
  body: {
    error: {
      type: "blocked",
      message:
        "Debit cards are not accepted for this transaction, please try a different card",
      code: "HE_03",
      reason: "Blocked",
    },
  },
};

export const blockedPaymentErrorBodyForCardSubtype = {
  status: 200,
  expectBlockedPayment: true,
  body: {
    error: {
      type: "blocked",
      message:
        "This card is not accepted for this transaction, please try a different card",
      code: "HE_03",
      reason: "Blocked",
    },
  },
};

export const blockedPaymentErrorBodyForBinUnavailable = {
  status: 200,
  expectBlockedPayment: true,
  body: {
    error: {
      type: "blocked",
      message:
        "We couldn't verify this card's information, please try a different card",
      code: "HE_03",
      reason: "Blocked",
    },
  },
};

export const connectorDetails = {
  real_time_payment_pm: {
    PaymentIntent: {
      Request: {
        currency: "IDR",
        amount: 10000,
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    Qris: {
      Request: {
        payment_method: "real_time_payment",
        payment_method_type: "qris",
        payment_method_data: {
          real_time_payment: {
            qris: {},
          },
        },
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    QrisMandate: {
      Request: {
        payment_method: "real_time_payment",
        payment_method_type: "qris",
        payment_method_data: {
          real_time_payment: {
            qris: {},
          },
        },
        billing: billingDetails,
        setup_future_usage: "off_session",
        mandate_data: {
          customer_acceptance: {
            acceptance_type: "online",
            accepted_at: "2026-07-13T18:09:53Z",
            online: {
              ip_address: "127.0.0.1",
              user_agent: "test-agent",
            },
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
  },
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "IDR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        amount: 6000000,
        billing: billingDetails,
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
        currency: "IDR",
        amount: 6000000,
        shipping_cost: 100,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          shipping_cost: 100,
          amount: 6000000,
        },
      },
    },
    PaymentConfirmWithShippingCost: {
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          shipping_cost: 100,
          amount: 6000000,
        },
      },
    },
    No3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        amount: 6000000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    No3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        amount: 6000000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingDetails,
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
        amount: 2000000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    manualPaymentRefund: {
      Request: {
        amount: 6000000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    "3DSAutoCapture": {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successful3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "on_session",
          payment_method_data: paymentMethodData3ds,
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        mandate_data: multiUseMandateData,
        billing: billingDetails,
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
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        amount: 6000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        mandate_data: multiUseMandateData,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        // Xendit returns "pending" status on authorize and requires PSync to get the actual status,
        // hence the payment method status is not updated to "active" on retrieve
        skipPaymentMethodStatusAssertion: true,
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        // Xendit returns "pending" status on authorize and requires PSync to get the actual status,
        // hence the payment method status is not updated to "active" on retrieve
        skipPaymentMethodStatusAssertion: true,
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingDetails,
        currency: "IDR",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        // Xendit returns "pending" status on authorize and requires PSync to get the actual status,
        // hence the payment method status is not updated to "active" on retrieve
        skipPaymentMethodStatusAssertion: true,
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingDetails,
        currency: "IDR",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        // Xendit returns "pending" status on authorize and requires PSync to get the actual status,
        // hence the payment method status is not updated to "active" on retrieve
        skipPaymentMethodStatusAssertion: true,
      },
      Request: {
        amount: 6000000,
        setup_future_usage: "off_session",
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardConfirmManualCaptureOffSession: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        // Xendit returns "pending" status on authorize and requires PSync to get the actual status,
        // hence the payment method status is not updated to "active" on retrieve
        skipPaymentMethodStatusAssertion: true,
      },
      Request: {
        amount: 6000000,
        setup_future_usage: "off_session",
        currency: "IDR",
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        // Xendit returns "pending" status on authorize and requires PSync to get the actual status,
        // hence the payment method status is not updated to "active" on retrieve
        skipPaymentMethodStatusAssertion: true,
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        setup_future_usage: "on_session",
        customer_acceptance: customerAcceptance,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        amount: 6000000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        mandate_data: singleUseMandateData,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    PaymentIntentOffSession: {
      Request: {
        amount: 6000000,
        authentication_type: "no_three_ds",
        currency: "IDR",
        customer_acceptance: null,
        setup_future_usage: "off_session",
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
    "3DSManualCapture": {
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successful3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
          setup_future_usage: "on_session",
          payment_method_data: paymentMethodData3ds,
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        payment_method: "card",
        amount: 6000000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        mandate_data: singleUseMandateData,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateSingleUse3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successful3DSCardDetails,
        },
        currency: "IDR",
        mandate_data: singleUseMandateData,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateSingleUse3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successful3DSCardDetails,
        },
        currency: "IDR",
        mandate_data: singleUseMandateData,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "IDR",
        mandate_data: singleUseMandateData,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "IDR",
        mandate_data: singleUseMandateData,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "IDR",
        mandate_data: multiUseMandateData,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "IDR",
        mandate_data: multiUseMandateData,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateMultiUse3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successful3DSCardDetails,
        },
        currency: "IDR",
        mandate_data: multiUseMandateData,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MandateMultiUse3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successful3DSCardDetails,
        },
        currency: "IDR",
        mandate_data: multiUseMandateData,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MITAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        skipPaymentMethodStatusAssertion: true, // Xendit returns "pending" status on authorize and requires PSync to get the actual status, hence the payment method status is not updated to "active" on retrieve
      },
      Request: {
        amount: 6000000,
        currency: "IDR",
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MITAutoCaptureWithCustomerAcceptance: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        skipPaymentMethodStatusAssertion: true, // Xendit returns "pending" status on authorize and requires PSync to get the actual status, hence the payment method status is not updated to "active" on retrieve
      },
      Request: {
        currency: "IDR",
        amount: 6000000,
        customer_acceptance: {
          acceptance_type: "offline",
          accepted_at: "1963-05-03T04:07:52.723Z",
          online: {
            ip_address: "127.0.0.1",
            user_agent: "amet irure esse",
          },
        },
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MITManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        skipPaymentMethodStatusAssertion: true, // Xendit returns "pending" status on authorize and requires PSync to get the actual status, hence the payment method status is not updated to "active" on retrieve
      },
      Request: { amount: 6000000, currency: "IDR", billing: billingDetails },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    MITWithoutBillingAddress: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        skipPaymentMethodStatusAssertion: true, // Xendit returns "pending" status on authorize and requires PSync to get the actual status, hence the payment method status is not updated to "active" on retrieve
      },
      Request: {
        amount: 6000000,
        payment_channel: "telephone_order",
        billing: null,
        currency: "IDR",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          billing: null,
        },
      },
    },
    MITExceedingMandateAmount: {
      Request: {
        amount: 6000000,
        currency: "IDR",
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    Capture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
      },
      Request: {
        amount_to_capture: 6000000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          amount: 6000000,
          amount_capturable: 0,
          amount_received: 6000000,
        },
      },
    },
    PartialCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        TRIGGER_SKIP: true, // Skip this test as Partial Capture is not supported by Xendit.
      },
      Request: {
        amount_to_capture: 2000000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Payment method type not supported",
            code: "IR_19",
            reason: "Partial Capture is not supported by Xendit",
          },
        },
      },
    },
    Refund: {
      Request: {
        amount: 6000000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    VoidAfterConfirm: {
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Cancel/Void flow is not supported",
            code: "IR_19",
          },
        },
      },
    },
    PartialRefund: {
      Request: {
        amount: 2000000,
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
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        skipPaymentMethodStatusAssertion: true, // Xendit returns "pending" status on authorize and requires PSync to get the actual status, hence the payment method status is not updated to "active" on retrieve
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        skipPaymentMethodStatusAssertion: true, // Xendit returns "pending" status on authorize and requires PSync to get the actual status, hence the payment method status is not updated to "active" on retrieve
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    PaymentMethodIdMandate3DSAutoCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        skipPaymentMethodStatusAssertion: true, // Xendit returns "pending" status on authorize and requires PSync to get the actual status, hence the payment method status is not updated to "active" on retrieve
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successful3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    PaymentMethodIdMandate3DSManualCapture: {
      Configs: {
        DELAY: {
          STATUS: true,
          TIMEOUT: 3000,
        },
        skipPaymentMethodStatusAssertion: true, // Xendit returns "pending" status on authorize and requires PSync to get the actual status, hence the payment method status is not updated to "active" on retrieve
      },
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: successful3DSCardDetails,
          billing: billingDetails,
        },
        currency: "IDR",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    CaptureGreaterAmount: {
      Request: {
        amount_to_capture: 600000000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "amount_to_capture is greater than amount",
            code: "IR_06",
          },
        },
      },
    },
    RefundGreaterAmount: {
      Request: {
        amount: 60000000,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "The refund amount exceeds the amount captured",
            code: "IR_13",
          },
        },
      },
    },
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails, // no failure test card available for Xendit connector
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardUse3DSAutoCaptureOffSession: {
      Request: {
        amount: 6000000,
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successful3DSCardDetails,
        },
        currency: "IDR",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
        billing: billingDetails,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    PaymentWithBilling: {
      Request: {
        currency: "IDR",
        setup_future_usage: "on_session",
        billing: billingDetails,
        email: "hyperswitch.example@gmail.com",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
  },
  payment_method_blocking_pm: {
    BlockIssuingCountry: {
      Request: {
        currency: "IDR",
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "4000000000000002",
            card_exp_month: "03",
            card_exp_year: "30",
            card_holder_name: "joseph Doeeee",
            card_cvc: "737",
            card_network: "Visa",
          },
        },
        billing: billingDetails,
      },
      Response: blockedPaymentErrorBodyForIssuingCountry,
    },
    BlockCardType: {
      Request: {
        currency: "IDR",
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "4111111111111111",
            card_exp_month: "03",
            card_exp_year: "30",
            card_holder_name: "joseph Doeeee",
            card_cvc: "737",
            card_network: "Visa",
          },
        },
        billing: billingDetails,
      },
      Response: blockedPaymentErrorBodyForDebitCard,
    },
    BlockCardSubtype: {
      Request: {
        currency: "IDR",
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "378282246310005",
            card_exp_month: "03",
            card_exp_year: "30",
            card_holder_name: "joseph Doeeee",
            card_cvc: "737",
            card_network: "Visa",
          },
        },
        billing: billingDetails,
      },
      Response: blockedPaymentErrorBodyForCardSubtype,
    },
    BlockIfBinInfoUnavailable: {
      Request: {
        currency: "IDR",
        amount: 6000000,
        payment_method: "card",
        payment_method_data: {
          card: {
            card_number: "6304000000000000",
            card_exp_month: "03",
            card_exp_year: "30",
            card_holder_name: "joseph Doeeee",
            card_cvc: "737",
            card_network: "Visa",
          },
        },
        billing: billingDetails,
      },
      Response: blockedPaymentErrorBodyForBinUnavailable,
    },
  },
};
