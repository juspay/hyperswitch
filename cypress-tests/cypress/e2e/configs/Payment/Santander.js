import { getCustomExchange, getCurrency } from "./Modifiers";

const billingAddress = {
  address: {
    line1: "Rua Augusta",
    line2: "2000",
    line3: "Consolação",
    city: "São Paulo",
    state: "SP",
    zip: "01412-000",
    country: "BR",
    first_name: "joseph",
    last_name: "Doe",
  },
  phone: {
    number: "11991234567",
    country_code: "+55",
  },
};

const customerDocumentDetails = {
  document_details: {
    document_type: "cpf",
    document_number: "86665623580",
  },
};

const connectorMetadata = {
  santander: {
    pix_automatico_qr: {
      cit: {
        mandate_details: {
          start_date: "2026-06-01",
          end_date: "2027-06-01",
          periodicity: "mensal",
          max_mandate_amount: 10000,
        },
        retry_policy: false,
      },
    },
  },
};

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
    status: 500,
    body: {
      error: {
        type: "api",
        message: "Something went wrong",
        code: "HE_00",
      },
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
        "InstantBankTransferFinland",
        "InstantBankTransferPoland",
        "Ach",
      ];
      const skipConfirmMethods = ["Boleto"];
      if (unsupportedMethods.includes(paymentMethodType)) {
        return bankTransferSkipConfigs[paymentMethodType];
      }
      if (skipConfirmMethods.includes(paymentMethodType)) {
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
        payment_method: "bank_transfer",
        payment_method_type: "pix",
        payment_method_data: {
          bank_transfer: {
            pix: {
              cpf: "86665623580",
            },
          },
        },
        billing: billingAddress,
        customer: customerDocumentDetails,
        feature_metadata: {
          pix_additional_details: {
            immediate: {
              time: 3600,
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
    }),
    Boleto: getCustomExchange({
      Request: {
        payment_method: "voucher",
        payment_method_type: "boleto",
        payment_method_data: {
          voucher: {
            boleto: {
              social_security_number: "86665623580",
            },
          },
        },
        billing: billingAddress,
        customer: customerDocumentDetails,
        connector_metadata: {
          santander: {
            boleto: {
              document_kind: "NPC",
            },
          },
        },
        feature_metadata: {
          boleto_additional_details: {
            due_date: "2030-12-31",
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_customer_action",
        },
      },
    }),
    PixAutomatico: getCustomExchange({
      Configs: { TRIGGER_SKIP: true },
      Request: {
        payment_method: "bank_transfer",
        payment_method_type: "pix_automatico_qr",
        payment_method_data: {
          bank_transfer: {
            pix_automatico_qr: {
              cpf: "86665623580",
            },
          },
        },
        billing: billingAddress,
        customer: customerDocumentDetails,
        connector_metadata: connectorMetadata,
        feature_metadata: {
          pix_additional_details: {
            immediate: {
              time: 3600,
            },
          },
        },
        setup_future_usage: "off_session",
      },
      Response: {
        status: 501,
        body: {
          error: {
            type: "invalid_request",
          },
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
};
