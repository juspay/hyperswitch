import { getCustomExchange, getCurrency } from "./Modifiers";

const cardCreateSkipConfig = getCustomExchange({
  Configs: { TRIGGER_SKIP: true },
  Request: { currency: "BRL" },
  Response: {
    status: 200,
    body: {
      status: "requires_payment_method",
    },
  },
});

const cardConfirmSkipConfig = getCustomExchange({
  Configs: { TRIGGER_SKIP: true },
  Request: { currency: "BRL" },
  Response: {
    status: 200,
    body: {
      status: "requires_payment_method",
    },
  },
});

const bankTransferSkipConfigs = {
  InstantBankTransferFinland: getCustomExchange({
    Configs: { TRIGGER_SKIP: true },
    Request: { currency: "EUR" },
    Response: {
      status: 200,
      body: {
        status: "requires_payment_method",
      },
    },
  }),
  InstantBankTransferPoland: getCustomExchange({
    Configs: { TRIGGER_SKIP: true },
    Request: { currency: "PLN" },
    Response: {
      status: 200,
      body: {
        status: "requires_payment_method",
      },
    },
  }),
  Ach: getCustomExchange({
    Configs: { TRIGGER_SKIP: true },
    Request: { currency: "USD" },
    Response: {
      status: 200,
      body: {
        status: "requires_payment_method",
      },
    },
  }),
};

export const connectorDetails = {
  card_pm: {
    PaymentIntent: cardCreateSkipConfig,
    PaymentIntentOffSession: cardCreateSkipConfig,
    PaymentIntentWithShippingCost: cardCreateSkipConfig,
    PaymentIntentWithSessionExpiry: cardCreateSkipConfig,
    PaymentConfirmWithShippingCost: cardConfirmSkipConfig,
    SessionExpiredConfirmPayment: cardConfirmSkipConfig,
    "3DSManualCapture": cardConfirmSkipConfig,
    "3DSAutoCapture": cardConfirmSkipConfig,
    SessionToken: cardConfirmSkipConfig,
    No3DSManualCapture: cardConfirmSkipConfig,
    No3DSAutoCapture: cardConfirmSkipConfig,
    No3DSFailPayment: cardConfirmSkipConfig,
    ManualRetryPaymentDisabled: cardConfirmSkipConfig,
    ManualRetryPaymentEnabled: cardConfirmSkipConfig,
    ManualRetryPaymentCutoffExpired: cardConfirmSkipConfig,
    Capture: cardConfirmSkipConfig,
    PartialCapture: cardConfirmSkipConfig,
    Void: cardConfirmSkipConfig,
    VoidAfterConfirm: cardConfirmSkipConfig,
    Refund: cardConfirmSkipConfig,
    PartialRefund: cardConfirmSkipConfig,
    SyncRefund: cardConfirmSkipConfig,
    manualPaymentRefund: cardConfirmSkipConfig,
    manualPaymentPartialRefund: cardConfirmSkipConfig,
    PartialAuth: cardConfirmSkipConfig,
    MandateSingleUse3DSAutoCapture: cardConfirmSkipConfig,
    MandateSingleUse3DSManualCapture: cardConfirmSkipConfig,
    MandateSingleUseNo3DSAutoCapture: cardConfirmSkipConfig,
    MandateSingleUseNo3DSManualCapture: cardConfirmSkipConfig,
    MandateMultiUseNo3DSAutoCapture: cardConfirmSkipConfig,
    MandateMultiUseNo3DSManualCapture: cardConfirmSkipConfig,
    MandateMultiUse3DSAutoCapture: cardConfirmSkipConfig,
    MandateMultiUse3DSManualCapture: cardConfirmSkipConfig,
    ZeroAuthMandate: cardConfirmSkipConfig,
    ZeroAuthPaymentIntent: cardCreateSkipConfig,
    ZeroAuthConfirmPayment: cardConfirmSkipConfig,
    SaveCardUseNo3DSAutoCapture: cardConfirmSkipConfig,
    SaveCardUseNo3DSAutoCaptureOffSession: cardConfirmSkipConfig,
    SaveCardUse3DSAutoCaptureOffSession: cardConfirmSkipConfig,
    SaveCardUseNo3DSManualCaptureOffSession: cardConfirmSkipConfig,
    SaveCardConfirmAutoCaptureOffSession: cardConfirmSkipConfig,
    SaveCardConfirmManualCaptureOffSession: cardConfirmSkipConfig,
    SaveCardConfirmAutoCaptureOffSessionWithoutBilling: cardConfirmSkipConfig,
    SaveCardUseNo3DSManualCapture: cardConfirmSkipConfig,
    PaymentMethod: cardConfirmSkipConfig,
    PaymentMethodIdMandateNo3DSAutoCapture: cardConfirmSkipConfig,
    PaymentMethodIdMandateNo3DSManualCapture: cardConfirmSkipConfig,
    PaymentMethodIdMandate3DSAutoCapture: cardConfirmSkipConfig,
    PaymentMethodIdMandate3DSManualCapture: cardConfirmSkipConfig,
    InvalidCardNumber: cardConfirmSkipConfig,
    InvalidExpiryMonth: cardConfirmSkipConfig,
    InvalidExpiryYear: cardConfirmSkipConfig,
    InvalidCardCvv: cardConfirmSkipConfig,
    InvalidCurrency: cardConfirmSkipConfig,
    InvalidCaptureMethod: cardConfirmSkipConfig,
    InvalidPaymentMethod: cardConfirmSkipConfig,
    InvalidAmountToCapture: cardConfirmSkipConfig,
    MissingRequiredParam: cardConfirmSkipConfig,
    PaymentIntentErrored: cardConfirmSkipConfig,
    CaptureGreaterAmount: cardConfirmSkipConfig,
    CaptureCapturedAmount: cardConfirmSkipConfig,
    ConfirmSuccessfulPayment: cardConfirmSkipConfig,
    RefundGreaterAmount: cardConfirmSkipConfig,
    MITAutoCapture: cardConfirmSkipConfig,
    MITWithoutBillingAddress: cardConfirmSkipConfig,
    MITWithLimitedCardData: cardConfirmSkipConfig,
    PartnerMerchantIdentifier: cardConfirmSkipConfig,
    PaymentWithoutBilling: cardConfirmSkipConfig,
    PaymentWithBilling: cardConfirmSkipConfig,
    PaymentWithFullName: cardConfirmSkipConfig,
    PaymentWithBillingEmail: cardConfirmSkipConfig,
    DuplicatePaymentID: cardConfirmSkipConfig,
    DuplicateRefundID: cardConfirmSkipConfig,
    InvalidPublishableKey: cardConfirmSkipConfig,
    DDCRaceConditionServerSide: cardConfirmSkipConfig,
    DDCRaceConditionClientSide: cardConfirmSkipConfig,
    PaymentIntentWithInstallments: cardConfirmSkipConfig,
    CardInstallmentConfirm: cardConfirmSkipConfig,
    PaymentIntentWithInstallmentsAndConfirmTrue: cardConfirmSkipConfig,
    external_three_ds: cardConfirmSkipConfig,
    UseBillingAsPaymentMethodBilling: cardConfirmSkipConfig,
    UseBillingAsPaymentMethodBillingDisabled: cardConfirmSkipConfig,
    ConnectorMetadata: cardConfirmSkipConfig,
  },
  bank_transfer_pm: {
    PaymentIntent: (paymentMethodType) => {
      const unsupportedMethods = [
        "Pix",
        "Boleto",
        "PixAutomatico",
        "InstantBankTransferFinland",
        "InstantBankTransferPoland",
        "Ach",
      ];
      if (unsupportedMethods.includes(paymentMethodType)) {
        return getCustomExchange({
          Configs: { TRIGGER_SKIP: true },
          Request: { currency: getCurrency(paymentMethodType) },
          Response: {
            status: 200,
            body: {
              status: "requires_payment_method",
            },
          },
        });
      }
      return getCustomExchange({
        Request: { currency: getCurrency(paymentMethodType) },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      });
    },
    Pix: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        currency: "BRL",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    Boleto: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        currency: "BRL",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    PixAutomatico: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        currency: "BRL",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    // PixAutomaticoPush skipped: requires MIT connector_metadata with receiver_details
    // (branch_code, account_number, account_type) not available in test environment
    PixAutomaticoPush: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        currency: "BRL",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    // PixAutomaticoQr skipped: requires MIT connector_metadata with receiver_details
    // (branch_code, account_number, account_type) not available in test environment
    PixAutomaticoQr: getCustomExchange({
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        currency: "BRL",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    InstantBankTransferFinland:
      bankTransferSkipConfigs.InstantBankTransferFinland,
    InstantBankTransferPoland:
      bankTransferSkipConfigs.InstantBankTransferPoland,
    Ach: bankTransferSkipConfigs.Ach,
  },
  bank_redirect_pm: {
    PaymentIntent: (paymentMethodType) =>
      getCustomExchange({
        Configs: { TRIGGER_SKIP: true },
        Request: { currency: getCurrency(paymentMethodType) },
        Response: {
          status: 200,
          body: {
            status: "requires_payment_method",
          },
        },
      }),
    Blik: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { currency: "PLN" },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    Eps: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { currency: "EUR" },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    Ideal: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { currency: "EUR" },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    Sofort: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { currency: "EUR" },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    Przelewy24: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { currency: "PLN" },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    OpenBankingUk: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { currency: "GBP" },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    OnlineBankingFpx: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { currency: "MYR" },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
    Interac: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: { currency: "CAD" },
      Response: {
        status: 200,
        body: {
          status: "requires_payment_method",
        },
      },
    }),
  },
};
