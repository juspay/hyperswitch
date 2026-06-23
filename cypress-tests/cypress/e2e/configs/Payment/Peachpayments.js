import { customerAcceptance } from "./Commons";
import { getCurrency, getCustomExchange } from "./Modifiers";

const successfulNo3DSCardDetails = {
  card_number: "5200000000000015",
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

const failedNo3DSCardDetails = {
  card_number: "4111111111111111",
  card_exp_month: "08",
  card_exp_year: "30",
  card_holder_name: "joseph Doe",
  card_cvc: "999",
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

// ============================================================================
// Peach Payments APMs (Payments API)
//
// Every brand is a distinct payment_method_type. Redirect brands respond with
// requires_customer_action and a redirect_to_url next action. 1ForYou is a
// synchronous voucher flow (no redirect).
// ============================================================================

const zaBillingAddress = {
  address: {
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "Johannesburg",
    state: "Gauteng",
    zip: "2000",
    country: "ZA",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "711111200",
    country_code: "+27",
  },
  email: "peach.apm@example.com",
};

const keBillingAddress = {
  address: {
    line1: "1467",
    line2: "Kenyatta Avenue",
    city: "Nairobi",
    state: "Nairobi",
    zip: "00100",
    country: "KE",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "712345678",
    country_code: "+254",
  },
  email: "peach.apm@example.com",
};

const muBillingAddress = {
  address: {
    line1: "1467",
    line2: "Royal Road",
    city: "Port Louis",
    state: "Port Louis",
    zip: "11328",
    country: "MU",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "51100000",
    country_code: "+230",
  },
  email: "peach.apm@example.com",
};

const apmRedirectResponse = {
  status: 200,
  body: {
    status: "requires_customer_action",
  },
};

const bankTransferPm = {
  PaymentIntent: (paymentMethodType) =>
    getCustomExchange({
      Request: {
        currency: getCurrency(paymentMethodType),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
  // Capitec Pay defaults to the cellphone identifier sourced from billing.phone.
  // The sandbox requires a Capitec-registered test identity; arbitrary numbers
  // return 800.900.300 "invalid authentication information"
  CapitecPay: getCustomExchange({
    Configs: {
      TRIGGER_SKIP: true,
    },
    Request: {
      payment_method: "bank_transfer",
      payment_method_type: "capitec_pay",
      payment_method_data: {
        bank_transfer: {
          capitec_pay: {
            account_type: "cellphone",
          },
        },
      },
      billing: zaBillingAddress,
      currency: "ZAR",
    },
    Response: apmRedirectResponse,
  }),
  // PayShap requires a bank selection; the proxy id is the billing phone
  PayShap: getCustomExchange({
    Request: {
      payment_method: "bank_transfer",
      payment_method_type: "pay_shap",
      payment_method_data: {
        bank_transfer: {
          pay_shap: {
            bank: "nedbank",
          },
        },
      },
      billing: zaBillingAddress,
      currency: "ZAR",
    },
    Response: apmRedirectResponse,
  }),
  // APMs support automatic capture only; manual capture is rejected upfront
  PayShapManualCapture: getCustomExchange({
    Request: {
      payment_method: "bank_transfer",
      payment_method_type: "pay_shap",
      payment_method_data: {
        bank_transfer: {
          pay_shap: {
            bank: "nedbank",
          },
        },
      },
      billing: zaBillingAddress,
      currency: "ZAR",
    },
    Response: {
      status: 400,
      body: {
        error: {
          type: "invalid_request",
        },
      },
    },
  }),
  NedbankDirectEft: getCustomExchange({
    Request: {
      payment_method: "bank_transfer",
      payment_method_type: "nedbank_direct_eft",
      payment_method_data: {
        bank_transfer: {
          nedbank_direct_eft: {},
        },
      },
      billing: zaBillingAddress,
      currency: "ZAR",
    },
    Response: apmRedirectResponse,
  }),
  PeachEft: getCustomExchange({
    Request: {
      payment_method: "bank_transfer",
      payment_method_type: "peach_eft",
      payment_method_data: {
        bank_transfer: {
          peach_eft: {},
        },
      },
      billing: zaBillingAddress,
      currency: "ZAR",
    },
    Response: apmRedirectResponse,
  }),
};

const payLaterPm = {
  PaymentIntent: (paymentMethodType) =>
    getCustomExchange({
      Request: {
        currency: getCurrency(paymentMethodType),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
  Payflex: getCustomExchange({
    Request: {
      payment_method: "pay_later",
      payment_method_type: "payflex",
      payment_method_data: {
        pay_later: {
          payflex_redirect: {},
        },
      },
      billing: zaBillingAddress,
      currency: "ZAR",
    },
    Response: apmRedirectResponse,
  }),
  // The sandbox ZeroPay acquirer is intermittently unavailable
  // (900.100.100 "unexpected communication error with connector/acquirer")
  ZeroPay: getCustomExchange({
    Configs: {
      TRIGGER_SKIP: true,
    },
    Request: {
      payment_method: "pay_later",
      payment_method_type: "zero_pay",
      payment_method_data: {
        pay_later: {
          zero_pay_redirect: {},
        },
      },
      billing: zaBillingAddress,
      currency: "ZAR",
      amount: 6000, // ZeroPay enforces a minimum of R30.00
    },
    Response: apmRedirectResponse,
  }),
  Float: getCustomExchange({
    Request: {
      payment_method: "pay_later",
      payment_method_type: "float",
      payment_method_data: {
        pay_later: {
          float_redirect: {},
        },
      },
      billing: zaBillingAddress,
      currency: "ZAR",
    },
    Response: apmRedirectResponse,
  }),
  HappyPay: getCustomExchange({
    Request: {
      payment_method: "pay_later",
      payment_method_type: "happy_pay",
      payment_method_data: {
        pay_later: {
          happy_pay_redirect: {},
        },
      },
      billing: zaBillingAddress,
      currency: "ZAR",
    },
    Response: apmRedirectResponse,
  }),
  // Mobicred requires real account credentials (billing email + password)
  Mobicred: getCustomExchange({
    Configs: {
      TRIGGER_SKIP: true,
    },
    Request: {
      payment_method: "pay_later",
      payment_method_type: "mobicred",
      payment_method_data: {
        pay_later: {
          mobicred_redirect: {
            password: "mobicred-test-password",
          },
        },
      },
      billing: zaBillingAddress,
      currency: "ZAR",
    },
    Response: apmRedirectResponse,
  }),
  // The sandbox RCS acquirer is intermittently unavailable
  // (900.100.203 "error on the internal gateway")
  Rcs: getCustomExchange({
    Configs: {
      TRIGGER_SKIP: true,
    },
    Request: {
      payment_method: "pay_later",
      payment_method_type: "rcs",
      payment_method_data: {
        pay_later: {
          rcs_redirect: {
            // RCS test store card number from the Peach Payments API collection
            card_number: "5614750003000013655",
          },
        },
      },
      billing: zaBillingAddress,
      currency: "ZAR",
    },
    Response: apmRedirectResponse,
  }),
  // A+ Store Cards availability on the Payments API is entity-dependent;
  // enable once the merchant's Peach account has the APLUS channel
  APlus: getCustomExchange({
    Configs: {
      TRIGGER_SKIP: true,
    },
    Request: {
      payment_method: "pay_later",
      payment_method_type: "a_plus",
      payment_method_data: {
        pay_later: {
          a_plus_redirect: {},
        },
      },
      billing: zaBillingAddress,
      currency: "ZAR",
    },
    Response: apmRedirectResponse,
  }),
};

const walletPm = {
  PaymentIntent: (paymentMethodType) =>
    getCustomExchange({
      Request: {
        currency: getCurrency(paymentMethodType),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
  // M-PESA requires a Kenya (KES) entity on the merchant account
  Mpesa: getCustomExchange({
    Configs: {
      TRIGGER_SKIP: true,
    },
    Request: {
      payment_method: "wallet",
      payment_method_type: "mpesa",
      payment_method_data: {
        wallet: {
          mpesa_redirect: {},
        },
      },
      billing: keBillingAddress,
      currency: "KES",
    },
    Response: apmRedirectResponse,
  }),
  // blink by Emtel requires a Mauritius (MUR) entity on the merchant account
  BlinkByEmtel: getCustomExchange({
    Configs: {
      TRIGGER_SKIP: true,
    },
    Request: {
      payment_method: "wallet",
      payment_method_type: "blink_by_emtel",
      payment_method_data: {
        wallet: {
          blink_by_emtel_redirect: {},
        },
      },
      billing: muBillingAddress,
      currency: "MUR",
    },
    Response: apmRedirectResponse,
  }),
  // MCB Juice requires a Mauritius (MUR) entity on the merchant account
  McbJuice: getCustomExchange({
    Configs: {
      TRIGGER_SKIP: true,
    },
    Request: {
      payment_method: "wallet",
      payment_method_type: "mcb_juice",
      payment_method_data: {
        wallet: {
          mcb_juice_redirect: {},
        },
      },
      billing: muBillingAddress,
      currency: "MUR",
    },
    Response: apmRedirectResponse,
  }),
  // Scan to Pay (formerly Masterpass) QR payments
  ScanToPay: getCustomExchange({
    Request: {
      payment_method: "wallet",
      payment_method_type: "scan_to_pay",
      payment_method_data: {
        wallet: {
          scan_to_pay_redirect: {},
        },
      },
      billing: zaBillingAddress,
      currency: "ZAR",
    },
    Response: apmRedirectResponse,
  }),
  // MauCAS QR requires a Mauritius (MUR) entity on the merchant account
  Maucas: getCustomExchange({
    Configs: {
      TRIGGER_SKIP: true,
    },
    Request: {
      payment_method: "wallet",
      payment_method_type: "maucas",
      payment_method_data: {
        wallet: {
          maucas_redirect: {},
        },
      },
      billing: muBillingAddress,
      currency: "MUR",
    },
    Response: apmRedirectResponse,
  }),
};

const voucherPm = {
  PaymentIntent: (paymentMethodType) =>
    getCustomExchange({
      Request: {
        currency: getCurrency(paymentMethodType),
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
  // 1ForYou is synchronous and needs a real voucher PIN purchased at retail
  OneForYou: getCustomExchange({
    Configs: {
      TRIGGER_SKIP: true,
    },
    Request: {
      payment_method: "voucher",
      payment_method_type: "one_for_you",
      payment_method_data: {
        voucher: {
          one_for_you: {
            voucher_pin: "1234567890123456",
          },
        },
      },
      billing: zaBillingAddress,
      currency: "ZAR",
    },
    Response: {
      status: 200,
      body: {
        status: "succeeded",
      },
    },
  }),
};

const cryptoPm = {
  PaymentIntent: getCustomExchange({
    Request: {
      currency: "ZAR",
      customer_acceptance: null,
      setup_future_usage: "on_session",
    },
    Response: {
      status: 200,
      body: {
        status: "requires_payment_method",
      },
    },
  }),
  MoneyBadger: getCustomExchange({
    Request: {
      payment_method: "crypto",
      payment_method_type: "money_badger",
      payment_method_data: {
        crypto: {},
      },
      billing: zaBillingAddress,
      currency: "ZAR",
    },
    Response: apmRedirectResponse,
  }),
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
    No3DSFailPayment: {
      Request: {
        payment_method: "card",
        payment_method_data: {
          card: failedNo3DSCardDetails,
        },
        currency: "USD",
        customer_acceptance: null,
        setup_future_usage: "on_session",
      },
      Response: {
        status: 200,
        body: {
          status: "failed",
          error_code: "15",
          error_message: "No such issuer (invalid IIN)",
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
    MITWithLimitedCardData: {
      Request: {},
      Response: {
        status: 200,
        body: {
          status: "succeeded",
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
  bank_transfer_pm: bankTransferPm,
  pay_later_pm: payLaterPm,
  wallet_pm: walletPm,
  voucher_pm: voucherPm,
  crypto_pm: cryptoPm,
};
