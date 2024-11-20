const successfulNo3DSCardDetails = {
  card_number: "1111222233334444",
  card_exp_month: "05",
  card_exp_year: "27",
  card_holder_name: "joseph Doe",
  card_cvc: "222",
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
          status: "processing",
        },
      },
    },
    Capture: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6500,
          amount_capturable: 6500,
          amount_received: null,
        },
      },
    },
    PartialCapture: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "processing",
          amount: 6500,
          amount_capturable: 6500,
          amount_received: null,
        },
      },
    },
    Refund: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
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
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
        },
      },
    },
    SyncRefund: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        customer_acceptance: null,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
            code: "IR_06"
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
            message: "Json deserialize error: unknown variant `United`, expected one of `AED`, `AFN`, `ALL`, `AMD`, `ANG`, `AOA`, `ARS`, `AUD`, `AWG`, `AZN`, `BAM`, `BBD`, `BDT`, `BGN`, `BHD`, `BIF`, `BMD`, `BND`, `BOB`, `BRL`, `BSD`, `BTN`, `BWP`, `BYN`, `BZD`, `CAD`, `CDF`, `CHF`, `CLP`, `CNY`, `COP`, `CRC`, `CUP`, `CVE`, `CZK`, `DJF`, `DKK`, `DOP`, `DZD`, `EGP`, `ERN`, `ETB`, `EUR`, `FJD`, `FKP`, `GBP`, `GEL`, `GHS`, `GIP`, `GMD`, `GNF`, `GTQ`, `GYD`, `HKD`, `HNL`, `HRK`, `HTG`, `HUF`, `IDR`, `ILS`, `INR`, `IQD`, `IRR`, `ISK`, `JMD`, `JOD`, `JPY`, `KES`, `KGS`, `KHR`, `KMF`, `KPW`, `KRW`, `KWD`, `KYD`, `KZT`, `LAK`, `LBP`, `LKR`, `LRD`, `LSL`, `LYD`, `MAD`, `MDL`, `MGA`, `MKD`, `MMK`, `MNT`, `MOP`, `MRU`, `MUR`, `MVR`, `MWK`, `MXN`, `MYR`, `MZN`, `NAD`, `NGN`, `NIO`, `NOK`, `NPR`, `NZD`, `OMR`, `PAB`, `PEN`, `PGK`, `PHP`, `PKR`, `PLN`, `PYG`, `QAR`, `RON`, `RSD`, `RUB`, `RWF`, `SAR`, `SBD`, `SCR`, `SDG`, `SEK`, `SGD`, `SHP`, `SLE`, `SLL`, `SOS`, `SRD`, `SSP`, `STN`, `SVC`, `SYP`, `SZL`, `THB`, `TJS`, `TMT`, `TND`, `TOP`, `TRY`, `TTD`, `TWD`, `TZS`, `UAH`, `UGX`, `USD`, `UYU`, `UZS`, `VES`, `VND`, `VUV`, `WST`, `XAF`, `XCD`, `XOF`, `XPF`, `YER`, `ZAR`, `ZMW`, `ZWL`",
            code: "IR_06"
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
            message: "Json deserialize error: unknown variant `auto`, expected one of `automatic`, `manual`, `manual_multiple`, `scheduled`",
            code: "IR_06"
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
            message: "Json deserialize error: unknown variant `this_supposed_to_be_a_card`, expected one of `card`, `card_redirect`, `pay_later`, `wallet`, `bank_redirect`, `bank_transfer`, `crypto`, `bank_debit`, `reward`, `real_time_payment`, `upi`, `voucher`, `gift_card`, `open_banking`",
            code: "IR_06"
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
            message: "A payment token or payment method data is required",
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
