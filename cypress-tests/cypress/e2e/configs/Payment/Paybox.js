const successfulNo3DSCardDetails = {
  card_number: "1111222233334444",
  card_exp_month: "05",
  card_exp_year: "27",
  card_holder_name: "joseph Doe",
  card_cvc: "222",
};

const successfulThreeDSTestCardDetails = {
  card_number: "4000000000001091",
  card_exp_month: "01",
  card_exp_year: "50",
  card_holder_name: "joseph Doe",
  card_cvc: "123",
};

const customerAcceptance = {
  acceptance_type: "offline",
  accepted_at: "1963-05-03T04:07:52.723Z",
  online: {
    ip_address: "125.0.0.1",
    user_agent: "amet irure esse",
  },
};

const singleUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    single_use: {
      amount: 6000,
      currency: "EUR",
    },
  },
};

const multiUseMandateData = {
  customer_acceptance: customerAcceptance,
  mandate_type: {
    multi_use: {
      amount: 6000,
      currency: "EUR",
    },
  },
};

const captureNotSupported = {
  type: "invalid_request",
  message: "Payment method type not supported",
  code: "IR_19",
  reason:
    "Capture Not allowed in case of Creating the Subscriber is not supported by Paybox",
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
        },
      },
    },
    PaymentIntentOffSession: {
      Request: {
        currency: "EUR",
        amount: 6000,
        authentication_type: "no_three_ds",
        customer_acceptance: null,
        setup_future_usage: "off_session",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        currency: "EUR",
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
          status: "requires_capture",
        },
      },
    },
    No3DSAutoCapture: {
      Request: {
        // Auto capture with different currency, so we need to pass currency in here
        currency: "EUR",
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
    VoidAfterConfirm: {
      Request: {},
      Response: {
        status: 501,
        body: {
          status: "cancelled",
          error: {
            type: "invalid_request",
            message: "Cancel/Void flow is not implemented",
            code: "IR_00",
          },
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
          status: "pending",
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
          status: "pending",
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
    MandateSingleUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 400,
        body: {
          error: captureNotSupported,
        },
      },
    },
    MandateSingleUse3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: singleUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
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
        status: 400,
        body: {
          error: captureNotSupported,
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
    MandateMultiUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          error: captureNotSupported,
        },
      },
    },
    MandateMultiUseNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MandateMultiUse3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 400,
        body: {
          error: captureNotSupported,
        },
      },
    },
    MandateMultiUse3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: multiUseMandateData,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MITAutoCapture: {
      Request: {
        currency: "EUR",
        amount: 6000,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    MITManualCapture: {
      Request: {
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 400,
        body: {
          error: captureNotSupported,
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    PaymentMethodIdMandate3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "EUR",
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 400,
        body: {
          error: captureNotSupported,
        },
      },
    },
    PaymentMethodIdMandate3DSManualCapture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        mandate_data: null,
        authentication_type: "three_ds",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    },
    ZeroAuthMandate: {
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
          status: "processing",
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "EUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          setup_future_usage: "off_session",
        },
      },
    },
    ZeroAuthConfirmPayment: {
      Request: {
        payment_type: "setup_mandate",
        payment_method: "card",
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
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
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        setup_future_usage: "off_session",
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 400,
        body: {
          error: captureNotSupported,
        },
      },
    },
    SaveCardUseNo3DSManualCaptureOffSession: {
      Request: {
        payment_method: "card",
        payment_method_type: "debit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
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
    SaveCardConfirmAutoCaptureOffSession: {
      Request: {
        setup_future_usage: "off_session",
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
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
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
    InvalidCardNumber: {
      Request: {
        currency: "EUR",
        payment_method: "card",
        payment_method_type: "debit",
        setup_future_usage: "on_session",
        payment_method_data: {
          card: {
            card_number: "123456",
            card_exp_month: "10",
            card_exp_year: "50",
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
        currency: "EUR",
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
        currency: "EUR",
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
        currency: "EUR",
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
              "Json deserialize error: unknown variant `United`, expected one of `AED`, `AFN`, `ALL`, `AMD`, `ANG`, `AOA`, `ARS`, `AUD`, `AWG`, `AZN`, `BAM`, `BBD`, `BDT`, `BGN`, `BHD`, `BIF`, `BMD`, `BND`, `BOB`, `BRL`, `BSD`, `BTN`, `BWP`, `BYN`, `BZD`, `CAD`, `CDF`, `CHF`, `CLP`, `CNY`, `COP`, `CRC`, `CUP`, `CVE`, `CZK`, `DJF`, `DKK`, `DOP`, `DZD`, `EGP`, `ERN`, `ETB`, `EUR`, `FJD`, `FKP`, `GBP`, `GEL`, `GHS`, `GIP`, `GMD`, `GNF`, `GTQ`, `GYD`, `HKD`, `HNL`, `HRK`, `HTG`, `HUF`, `IDR`, `ILS`, `INR`, `IQD`, `IRR`, `ISK`, `JMD`, `JOD`, `JPY`, `KES`, `KGS`, `KHR`, `KMF`, `KPW`, `KRW`, `KWD`, `KYD`, `KZT`, `LAK`, `LBP`, `LKR`, `LRD`, `LSL`, `LYD`, `MAD`, `MDL`, `MGA`, `MKD`, `MMK`, `MNT`, `MOP`, `MRU`, `MUR`, `MVR`, `MWK`, `MXN`, `MYR`, `MZN`, `NAD`, `NGN`, `NIO`, `NOK`, `NPR`, `NZD`, `OMR`, `PAB`, `PEN`, `PGK`, `PHP`, `PKR`, `PLN`, `PYG`, `QAR`, `RON`, `RSD`, `RUB`, `RWF`, `SAR`, `SBD`, `SCR`, `SDG`, `SEK`, `SGD`, `SHP`, `SLE`, `SLL`, `SOS`, `SRD`, `SSP`, `STN`, `SVC`, `SYP`, `SZL`, `THB`, `TJS`, `TMT`, `TND`, `TOP`, `TRY`, `TTD`, `TWD`, `TZS`, `UAH`, `UGX`, `USD`, `UYU`, `UZS`, `VES`, `VND`, `VUV`, `WST`, `XAF`, `XCD`, `XOF`, `XPF`, `YER`, `ZAR`, `ZMW`, `ZWL`",
            code: "IR_06",
          },
        },
      },
    },
    InvalidCaptureMethod: {
      Request: {
        currency: "EUR",
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
        currency: "EUR",
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
        currency: "EUR",
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
        currency: "EUR",
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
        currency: "EUR",
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
        Request: {
          payment_method: "card",
          payment_method_data: {
            card: successfulNo3DSCardDetails,
          },
          currency: "EUR",
          customer_acceptance: null,
        },
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
    ConfirmSuccessfulPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "EUR",
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
  },
};
