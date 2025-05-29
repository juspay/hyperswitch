import { customerAcceptance } from "./Commons";

// Test card details for Fiserv SnapPay
const successfulNo3DSCardDetails = {
  card_number: "4111111111111111", // Visa test card from Fiserv documentation
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
};

const successfulThreeDSCardDetails = {
  card_number: "4637090000158588", // Visa test card from Fiserv documentation
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
};

const successfulMastercardDetails = {
  card_number: "5399104611689124", // MasterCard test card from Fiserv documentation
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
};

const failedCardDetails = {
  card_number: "4012888888881881", // Standard decline test card for Fiserv - "Do Not Honor" response
  card_exp_month: "12",
  card_exp_year: "30",
  card_holder_name: "Joseph Doe",
  card_cvc: "123",
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 8000,
      currency: "USD",
    },
  },
};

export const cardRequiredField = {
  "payment_method_data.card.card_number": {
    required_field: "payment_method_data.card.card_number",
    display_name: "card_number",
    field_type: "user_card_number",
    value: null,
  },
  "payment_method_data.card.card_exp_year": {
    required_field: "payment_method_data.card.card_exp_year",
    display_name: "card_exp_year",
    field_type: "user_card_expiry_year",
    value: null,
  },
  "payment_method_data.card.card_cvc": {
    required_field: "payment_method_data.card.card_cvc",
    display_name: "card_cvc",
    field_type: "user_card_cvc",
    value: null,
  },
  "payment_method_data.card.card_exp_month": {
    required_field: "payment_method_data.card.card_exp_month",
    display_name: "card_exp_month",
    field_type: "user_card_expiry_month",
    value: null,
  },
};

export const fullNameRequiredField = {
  "billing.address.last_name": {
    required_field: "payment_method_data.billing.address.last_name",
    display_name: "card_holder_name",
    field_type: "user_full_name",
    value: "Doe",
  },
  "billing.address.first_name": {
    required_field: "payment_method_data.billing.address.first_name",
    display_name: "card_holder_name",
    field_type: "user_full_name",
    value: "joseph",
  },
};

export const billingRequiredField = {};

const requiredFields = {
  payment_methods: [
    {
      payment_method: "card",
      payment_method_types: [
        {
          payment_method_type: "credit",
          card_networks: [
            {
              eligible_connectors: ["fiserv"],
            },
          ],
          required_fields: cardRequiredField,
        },
      ],
    },
  ],
};

const payment_method_data_3ds = {
  card: {
    last4: "1111",
    card_type: "CREDIT",
    card_network: "Visa",
    card_issuer: "JP Morgan",
    card_issuing_country: "INDIA",
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "30",
    card_holder_name: "Joseph Doe",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
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
    card_exp_month: "12",
    card_exp_year: "30",
    card_holder_name: "Joseph Doe",
    payment_checks: null,
    authentication_data: null,
  },
  billing: null,
};

const payment_method_data_mastercard = {
  card: {
    last4: "9124",
    card_type: "CREDIT",
    card_network: "Mastercard",
    card_issuer: "Test Bank",
    card_issuing_country: "UNITEDSTATES",
    card_isin: "539910",
    card_extended_bin: null,
    card_exp_month: "12",
    card_exp_year: "30",
    card_holder_name: "Joseph Doe",
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
    // Add Mastercard specific flow
    MastercardAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulMastercardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_mastercard,
        },
      },
    },
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
          setup_future_usage: "on_session",
        },
      },
    },
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          setup_future_usage: "on_session",
          payment_method_data: payment_method_data_3ds,
        },
      },
    },
    "3DSAutoCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
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
    No3DSAutoCapture: {
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
          payment_method: "card",
          payment_method_data: payment_method_data_no3ds,
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
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_no3ds,
        },
      },
    },
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
          error_message: "Unable to assign card to brand: Invalid",
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
      Request: {
        cancellation_reason: "VOID",
      },
      Response: {
        status: 200,
        body: {
          status: "cancelled",
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
    SyncPayment: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method: "card",
          payment_method_data: payment_method_data_no3ds,
        },
      },
    },
    MandateSingleUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    },
    MandateSingleUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          mandate_id: null,
          payment_method_data: payment_method_data_no3ds,
          payment_method: "card",
          connector: "fiserv",
        },
      },
    },
    MandateSingleUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          mandate_id: null,
          payment_method_data: payment_method_data_no3ds,
          payment_method: "card",
          connector: "fiserv",
        },
      },
    },
    MandateMultiUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          payment_method_data: payment_method_data_no3ds,
          payment_method: "card",
          connector: "fiserv",
        },
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          payment_method_data: payment_method_data_no3ds,
          payment_method: "card",
          connector: "fiserv",
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
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
    SaveCardUse3DSAutoCaptureOnSession: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        currency: "USD",
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
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardUse3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    PaymentIntentOffSession: {
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
    SaveCardConfirmAutoCaptureOffSession: {
      Request: {
        setup_future_usage: "off_session",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    SaveCardConfirmManualCaptureOffSession: {
      Request: {
        setup_future_usage: "off_session",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    SaveCardConfirmAutoCaptureOffSessionWithoutBilling: {
      Request: {
        setup_future_usage: "off_session",
        billing: null,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          billing: null,
        },
      },
    },
    SaveCardUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
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
    SaveCardUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        currency: "USD",
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
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    },
    PaymentMethodIdMandate3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    },
    PaymentMethodIdMandate3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSCardDetails,
        },
        currency: "USD",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    },
    MITWithoutBillingAddress: {
      Request: {
        billing: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          billing: null,
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    },
    ZeroAuthMandate: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Fiserv is not implemented",
            code: "IR_00",
          },
        },
      },
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
    },
    ZeroAuthConfirmPayment: {
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Fiserv is not implemented",
            code: "IR_00",
          },
        },
      },
    },
    MITAutoCapture: {
      // Fiserv does not support MIT payments with mandate_id
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
          mandate_id: null,
          payment_method: "card",
          payment_method_data: payment_method_data_no3ds,
          connector: "fiserv",
        },
      },
    },
    MITManualCapture: {
      // Fiserv does not support MIT payments with mandate_id
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
          mandate_id: null,
          payment_method: "card",
          payment_method_data: payment_method_data_no3ds,
          connector: "fiserv",
        },
      },
    },
  },
  // Add support for bank redirects if needed
  bank_redirect_pm: {
    PaymentIntent: {
      Request: {
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    },
  },

  // Error case configurations
  InvalidCardNumber: {
    Request: {
      currency: "USD",
      payment_method: "card",
      payment_method_type: "debit",
      setup_future_usage: "on_session",
      payment_method_data: {
        card: {
          card_number: "123456",
          card_exp_month: "10",
          card_exp_year: "25",
          card_holder_name: "joseph Doe",
          card_cvc: "123",
        },
      },
    },
    Response: {
      status: 400,
      body: {
        error: {
          error_type: "invalid_request",
          message: "Json deserialize error: invalid card number length",
          code: "IR_06",
        },
      },
    },
  },
  InvalidExpiryMonth: {
    Request: {
      currency: "USD",
      payment_method: "card",
      payment_method_type: "debit",
      setup_future_usage: "on_session",
      payment_method_data: {
        card: {
          card_number: "4242424242424242",
          card_exp_month: "00",
          card_exp_year: "2023",
          card_holder_name: "joseph Doe",
          card_cvc: "123",
        },
      },
    },
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          message: "Invalid Expiry Month",
          code: "IR_16",
        },
      },
    },
  },
  InvalidExpiryYear: {
    Request: {
      currency: "USD",
      payment_method: "card",
      payment_method_type: "debit",
      setup_future_usage: "on_session",
      payment_method_data: {
        card: {
          card_number: "4242424242424242",
          card_exp_month: "01",
          card_exp_year: "2023",
          card_holder_name: "joseph Doe",
          card_cvc: "123",
        },
      },
    },
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          message: "Invalid Expiry Year",
          code: "IR_16",
        },
      },
    },
  },
  InvalidCardCvv: {
    Request: {
      currency: "USD",
      payment_method: "card",
      payment_method_type: "debit",
      setup_future_usage: "on_session",
      payment_method_data: {
        card: {
          card_number: "4242424242424242",
          card_exp_month: "01",
          card_exp_year: "2023",
          card_holder_name: "joseph Doe",
          card_cvc: "123456",
        },
      },
    },
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          message: "Invalid card_cvc length",
          code: "IR_16",
        },
      },
    },
  },
  InvalidCurrency: {
    Request: {
      currency: "United",
      payment_method: "card",
      payment_method_type: "debit",
      setup_future_usage: "on_session",
      payment_method_data: {
        card: {
          card_number: "4242424242424242",
          card_exp_month: "01",
          card_exp_year: "2023",
          card_holder_name: "joseph Doe",
          card_cvc: "123456",
        },
      },
    },
    Response: {
      status: 400,
      body: {
        error: {
          error_type: "invalid_request",
          message:
            "Json deserialize error: unknown variant `United`, expected one of `AED`, `AFN`, `ALL`, `AMD`, `ANG`, `AOA`, `ARS`, `AUD`, `AWG`, `AZN`, `BAM`, `BBD`, `BDT`, `BGN`, `BHD`, `BIF`, `BMD`, `BND`, `BOB`, `BRL`, `BSD`, `BTN`, `BWP`, `BYN`, `BZD`, `CAD`, `CDF`, `CHF`, `CLF`, `CLP`, `CNY`, `COP`, `CRC`, `CUC`, `CUP`, `CVE`, `CZK`, `DJF`, `DKK`, `DOP`, `DZD`, `EGP`, `ERN`, `ETB`, `EUR`, `FJD`, `FKP`, `GBP`, `GEL`, `GHS`, `GIP`, `GMD`, `GNF`, `GTQ`, `GYD`, `HKD`, `HNL`, `HRK`, `HTG`, `HUF`, `IDR`, `ILS`, `INR`, `IQD`, `IRR`, `ISK`, `JMD`, `JOD`, `JPY`, `KES`, `KGS`, `KHR`, `KMF`, `KPW`, `KRW`, `KWD`, `KYD`, `KZT`, `LAK`, `LBP`, `LKR`, `LRD`, `LSL`, `LYD`, `MAD`, `MDL`, `MGA`, `MKD`, `MMK`, `MNT`, `MOP`, `MRU`, `MUR`, `MVR`, `MWK`, `MXN`, `MYR`, `MZN`, `NAD`, `NGN`, `NIO`, `NOK`, `NPR`, `NZD`, `OMR`, `PAB`, `PEN`, `PGK`, `PHP`, `PKR`, `PLN`, `PYG`, `QAR`, `RON`, `RSD`, `RUB`, `RWF`, `SAR`, `SBD`, `SCR`, `SDG`, `SEK`, `SGD`, `SHP`, `SLE`, `SLL`, `SOS`, `SRD`, `SSP`, `STD`, `STN`, `SVC`, `SYP`, `SZL`, `THB`, `TJS`, `TMT`, `TND`, `TOP`, `TRY`, `TTD`, `TWD`, `TZS`, `UAH`, `UGX`, `USD`, `UYU`, `UZS`, `VES`, `VND`, `VUV`, `WST`, `XAF`, `XCD`, `XOF`, `XPF`, `YER`, `ZAR`, `ZMW`, `ZWL`",
          code: "IR_06",
        },
      },
    },
  },
  InvalidCaptureMethod: {
    Request: {
      currency: "USD",
      capture_method: "auto",
      payment_method: "card",
      payment_method_type: "debit",
      setup_future_usage: "on_session",
      payment_method_data: {
        card: {
          card_number: "4242424242424242",
          card_exp_month: "01",
          card_exp_year: "2023",
          card_holder_name: "joseph Doe",
          card_cvc: "123456",
        },
      },
    },
    Response: {
      status: 400,
      body: {
        error: {
          error_type: "invalid_request",
          message:
            "Json deserialize error: unknown variant `auto`, expected one of `automatic`, `manual`, `manual_multiple`, `scheduled`",
          code: "IR_06",
        },
      },
    },
  },
  InvalidPaymentMethod: {
    Request: {
      currency: "USD",
      payment_method: "this_supposed_to_be_a_card",
      payment_method_type: "debit",
      setup_future_usage: "on_session",
      payment_method_data: {
        card: {
          card_number: "4242424242424242",
          card_exp_month: "01",
          card_exp_year: "2023",
          card_holder_name: "joseph Doe",
          card_cvc: "123456",
        },
      },
    },
    Response: {
      status: 400,
      body: {
        error: {
          error_type: "invalid_request",
          message:
            "Json deserialize error: unknown variant `this_supposed_to_be_a_card`, expected one of `card`, `card_redirect`, `pay_later`, `wallet`, `bank_redirect`, `bank_transfer`, `crypto`, `bank_debit`, `reward`, `real_time_payment`, `upi`, `voucher`, `gift_card`, `open_banking`, `mobile_payment`",
          code: "IR_06",
        },
      },
    },
  },
  InvalidAmountToCapture: {
    Request: {
      currency: "USD",
      amount_to_capture: 10000,
      payment_method: "card",
      payment_method_type: "debit",
      setup_future_usage: "on_session",
      payment_method_data: {
        card: {
          card_number: "4242424242424242",
          card_exp_month: "01",
          card_exp_year: "2026",
          card_holder_name: "joseph Doe",
          card_cvc: "123",
        },
      },
    },
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          message:
            "amount_to_capture contains invalid data. Expected format is amount_to_capture lesser than amount",
          code: "IR_05",
        },
      },
    },
  },
  MissingRequiredParam: {
    Request: {
      currency: "USD",
      payment_method_type: "debit",
      setup_future_usage: "on_session",
      payment_method_data: {
        card: {
          card_number: "4242424242424242",
          card_exp_month: "01",
          card_exp_year: "2026",
          card_holder_name: "joseph Doe",
          card_cvc: "123",
        },
      },
    },
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          message: "Missing required param: payment_method",
          code: "IR_04",
        },
      },
    },
  },
  PaymentIntentErrored: {
    Request: {
      currency: "USD",
    },
    Response: {
      status: 422,
      body: {
        error: {
          type: "invalid_request",
          message:
            "A payment token or payment method data or ctp service details is required",
          code: "IR_06",
        },
      },
    },
  },
  CaptureGreaterAmount: {
    Request: {
      amount_to_capture: 6000000,
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
  ThreeDSGreaterCapture: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: successfulThreeDSCardDetails,
      },
      currency: "USD",
    },
    Response: {
      status: 200,
      body: {
        status: "requires_capture",
        payment_method: "card",
        payment_method_data: payment_method_data_3ds,
        connector: "fiserv",
      },
    },
  },
  CaptureCapturedAmount: {
    Request: {
      amount_to_capture: 6000,
    },
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          message:
            "This Payment could not be captured because it has a payment.status of succeeded. The expected state is requires_capture, partially_captured_and_capturable, processing",
          code: "IR_14",
        },
      },
    },
  },
  ConfirmSuccessfulPayment: {
    Request: {
      payment_method: "card",
      payment_method_data: {
        card: successfulNo3DSCardDetails,
      },
      currency: "USD",
      customer_acceptance: null,
    },
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          message:
            "You cannot confirm this payment because it has status succeeded",
          code: "IR_16",
        },
      },
    },
  },
  RefundGreaterAmount: {
    Request: {
      amount: 6000000,
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
  DuplicatePaymentID: {
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
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          message:
            "The payment with the specified payment_id already exists in our records",
          code: "HE_01",
        },
      },
    },
  },
  DuplicateRefundID: {
    Request: {
      amount: 2000,
    },
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
          message:
            "Duplicate refund request. Refund already attempted with the refund ID",
          code: "HE_01",
        },
      },
    },
  },
  pm_list: {
    PmListResponse: {
      PmListNull: {
        payment_methods: [],
      },
      pmListDynamicFieldWithoutBilling: requiredFields,
      pmListDynamicFieldWithBilling: requiredFields,
      pmListDynamicFieldWithNames: requiredFields,
      pmListDynamicFieldWithEmail: requiredFields,
    },
  },
};
