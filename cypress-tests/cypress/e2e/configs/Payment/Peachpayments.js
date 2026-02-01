import { customerAcceptance } from "./Commons";
import { getCurrency, getCustomExchange } from "./Modifiers";

const successfulNo3DSCardDetails = {
  card_number: "5181030000183696",
  card_exp_month: "01",
  card_exp_year: "28",
  card_holder_name: "John",
  card_cvc: "576",
};

const successfulThreeDSTestCardDetails = {
  card_number: "5181030000183696",
  card_exp_month: "01",
  card_exp_year: "28",
  card_holder_name: "Joseph",
  card_cvc: "576",
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

const billingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    city: "San Francisco",
    state: "California",
    zip: "94122",
    country: "US",
    first_name: "Test",
    last_name: "User",
  },
  phone: {
    number: "9123456789",
    country_code: "+1",
  },
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: {
      Request: {
        currency: "USD",
        amount: 6000,
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
    PaymentIntentWithShippingCost: {
      Request: {
        currency: "USD",
        amount: 6000,
        shipping_cost: 50,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
          amount: 6000,
          shipping_cost: 50,
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
          shipping_cost: 50,
          amount_received: 6050,
          amount: 6000,
          net_amount: 6050,
        },
      },
    },
    "3DSManualCapture": {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            code: "IR_19",
            message: "Payment method type not supported",
            reason: "3DS flow is not supported by Peachpayments",
            type: "invalid_request",
          },
        },
      },
    },
    // 3DS automatic capture
    "3DSAutoCapture": {
      config: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: successfulThreeDSTestCardDetails,
        },
        amount: 6000,
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 400,
        body: {
          error: {
            code: "IR_19",
            message: "Payment method type not supported",
            reason: "automatic is not supported by peachpayments",
            type: "invalid_request",
          },
        },
      },
    },
    No3DSManualCapture: {
      Request: {
        payment_method: "card",
        amount: 6000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddress,
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
        payment_method: "card",
        amount: 6000,
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
        billing: billingAddress,
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
    SyncRefund: {
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    ZeroAuthMandate: {
      Response: {
        status: 501,
        body: {
          error: {
            code: "IR_00",
            message: "Setup Mandate flow for Peachpayments is not implemented",
            type: "invalid_request",
          },
        },
      },
    },
    ZeroAuthPaymentIntent: {
      Request: {
        amount: 0,
        setup_future_usage: "off_session",
        currency: "USD",
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
        payment_method_type: "credit",
        payment_method_data: {
          card: successfulNo3DSCardDetails,
        },
        mandate_data: null,
        customer_acceptance: customerAcceptance,
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
            message: "Setup Mandate flow for Peachpayments is not implemented",
            code: "IR_00",
          },
        },
      },
    },
    SaveCardUseNo3DSAutoCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        amount: 6000,
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
    SaveCardUseNo3DSManualCapture: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
        amount: 6000,
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
    SaveCardUseNo3DSManualCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "card",
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
    SaveCardConfirmManualCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
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
    SaveCardUseNo3DSAutoCaptureOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
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
          status: "succeeded",
        },
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
        },
      },
    },
    PaymentMethodIdMandateNo3DSManualCapture: {
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
      },
      Response: {
        status: 200,
        body: {
          status: "requires_capture",
        },
      },
    },
    MITAutoCapture: {
      config: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            code: "IR_19",
            message: "Payment method type not supported",
            reason: "automatic is not supported by peachpayments",
            type: "invalid_request",
          },
        },
      },
    },
    PaymentMethodIdMandateNo3DSAutoCapture: {
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
      },
      Response: {
        status: 200,
        body: {
          status: "succeeded",
        },
      },
    },
    PaymentIntentOffSession: {
      Configs: {
        TRIGGER_SKIP: true,
      },
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
  },
  bank_transfer_pm: {
    PaymentIntent: (paymentMethodType) => {
      // Skip unsupported payment method types
      const unsupportedTypes = [
        "Pix",
        "InstantBankTransferFinland",
        "InstantBankTransferPoland",
        "Ach",
      ];
      if (unsupportedTypes.includes(paymentMethodType)) {
        return getCustomExchange({
          Configs: {
            TRIGGER_SKIP: true,
          },
          Request: {
            currency: getCurrency(paymentMethodType),
          },
          Response: {
            status: 200,
            body: {
              status: "requires_payment_method",
            },
          },
        });
      }
      return getCustomExchange({
        Request: {
          currency: getCurrency(paymentMethodType),
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      });
    },
    // Skip unsupported bank transfer types
    Pix: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            message: "Payment method type not supported",
          },
        },
      },
    }),
    InstantBankTransferFinland: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            message: "Payment method type not supported",
          },
        },
      },
    }),
    InstantBankTransferPoland: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            message: "Payment method type not supported",
          },
        },
      },
    }),
    Ach: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {},
      Response: {
        status: 400,
        body: {
          error: {
            message: "Payment method type not supported",
          },
        },
      },
    }),
    // ============================================================================
    // PeachPayments South African APMs (ZAR)
    // ============================================================================

    // PayShap - South African instant bank transfer (default LocalBankTransfer)
    // Phone format: +27-XXXXXXXXX (with hyphen)
    LocalBankTransfer: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "PAYSHAP:NEDBANK",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            line3: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
          phone: {
            number: "711111200",
            country_code: "+27-",
          },
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // Peach EFT - Standard bank redirect
    PeachEft: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "PEACHEFT",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // Capitec Pay - Capitec bank instant payments
    // Uses ID number instead of phone: 1111111111214
    CapitecPay: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "CAPITECPAY",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
          phone: {
            number: "1111111111214",
            country_code: "",
          },
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // Payflex - Buy Now Pay Later (R10-R50,000)
    Payflex: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "PAYFLEX",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // ZeroPay - Buy Now Pay Later (minimum R30)
    ZeroPay: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "ZEROPAY",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // Float - Buy Now Pay Later
    Float: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "FLOAT",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // Happy Pay - Buy Now Pay Later
    HappyPay: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "HAPPYPAY",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // Masterpass / Scan to Pay - QR code payments
    Masterpass: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "MASTERPASS",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // MoneyBadger - Crypto payments
    MoneyBadger: getCustomExchange({
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "MONEYBADGER",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // 1Voucher - Voucher-based payment (requires PIN - may fail without)
    OneVoucher: getCustomExchange({
      Configs: {
        // May fail without voucher PIN
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "1FORYOU",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
          email: "test@example.com",
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // Mobicred - Alternative credit (requires account setup)
    Mobicred: getCustomExchange({
      Configs: {
        // May fail without account credentials
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "MOBICRED",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
          email: "test@example.com",
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // RCS Cards - Store/credit cards (amount 1.00 for test success)
    RCS: getCustomExchange({
      Configs: {
        // May require specific test setup
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "RCS",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
          email: "test@example.com",
        },
        currency: "ZAR",
        amount: 100, // R1.00 for test success
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // A+ Store Cards (requires clearingInstituteSessionId)
    APlus: getCustomExchange({
      Configs: {
        // Requires special session ID
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "APLUS",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
          email: "test@example.com",
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),

    // ============================================================================
    // PeachPayments Kenya APMs (KES)
    // ============================================================================

    // M-PESA - Mobile money (Kenya)
    // Phone format: +254-XXXXXXXXX (with hyphen)
    Mpesa: getCustomExchange({
      Configs: {
        // May require Kenya-specific entity
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "MPESA",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Kenyatta Avenue",
            city: "Nairobi",
            state: "Nairobi",
            zip: "00100",
            country: "KE",
            first_name: "john",
            last_name: "doe",
          },
          phone: {
            number: "712345678",
            country_code: "+254-",
          },
        },
        currency: "KES",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),

    // ============================================================================
    // PeachPayments Mauritius APMs (MUR)
    // ============================================================================

    // blink by Emtel - Digital wallet (Mauritius)
    // Phone format: 8-digit number starting with 5, NO country code
    BlinkByEmtel: getCustomExchange({
      Configs: {
        // May require Mauritius-specific entity
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "BLINKBYEMTEL",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Royal Road",
            city: "Port Louis",
            state: "Port Louis",
            zip: "11328",
            country: "MU",
            first_name: "john",
            last_name: "doe",
          },
          phone: {
            number: "51100000",
            country_code: "",
          },
        },
        currency: "MUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // MCB Juice - Digital wallet (Mauritius)
    // Phone format: 8-digit number starting with 5, NO country code
    McbJuice: getCustomExchange({
      Configs: {
        // May require Mauritius-specific entity
        TRIGGER_SKIP: true,
      },
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "local_bank_transfer",
        payment_method_data: {
          bank_transfer: {
            local_bank_transfer: {
              bank_code: "MCBJUICE",
            },
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Royal Road",
            city: "Port Louis",
            state: "Port Louis",
            zip: "11328",
            country: "MU",
            first_name: "john",
            last_name: "doe",
          },
          phone: {
            number: "51100000",
            country_code: "",
          },
        },
        currency: "MUR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
  },
  // BNPL payment methods
  pay_later_pm: {
    PaymentIntent: () =>
      getCustomExchange({
        Request: {
          currency: "ZAR",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      }),
    // Payflex BNPL
    Payflex: getCustomExchange({
      Request: {
        payment_method: "pay_later",
        payment_method_type: "pay_bright",
        payment_method_data: {
          pay_later: {
            pay_bright_redirect: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
  },
  // Wallet payment methods
  wallet_pm: {
    PaymentIntent: () =>
      getCustomExchange({
        Request: {
          currency: "ZAR",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      }),
    // Masterpass/Scan to Pay
    Masterpass: getCustomExchange({
      Request: {
        payment_method: "wallet",
        payment_method_type: "we_chat_pay",
        payment_method_data: {
          wallet: {
            we_chat_pay_redirect: {},
          },
        },
        billing: {
          address: {
            line1: "1467",
            line2: "Harrison Street",
            city: "Johannesburg",
            state: "Gauteng",
            zip: "2000",
            country: "ZA",
            first_name: "john",
            last_name: "doe",
          },
        },
        currency: "ZAR",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
  },
};
