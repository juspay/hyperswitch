const wallet_data = {
  telephone_number: "16608213349",
};

/*
 * PayPal payouts are wallet-only:
 *   - PayPal wallet (receiver resolved from email / telephone_number /
 *     paypal_id)
 *   - Venmo wallet (receiver resolved from telephone_number)
 * The connector's payout fulfill transformer (PaypalPayoutItem::try_from in
 * crates/hyperswitch_connectors/src/connectors/paypal/transformers.rs) only
 * accepts PayoutMethodData::Wallet; Card and Bank payout method data are
 * rejected with ConnectorError::NotSupported, which the router surfaces as
 * 501 IR_00 "Selected payment method through paypal is not implemented".
 *
 * card_pm / bank_transfer_pm sections are therefore intentionally NOT
 * defined here: paypal is excluded from the card payout spec via
 * CONNECTOR_LISTS.EXCLUDE.CARD_TEST and is not part of any BANK_TRANSFER_* /
 * SAVED_* inclusion list in Utils.js, so the generic card / bank transfer /
 * saved-payout specs (00003-00006) skip cleanly at the gate instead of
 * "passing" by asserting the Commons.js 501 not-implemented fallback.
 * Positive PayPal payout coverage lives in 00009-ClientAuthPayoutConfirm.cy.js
 * (wallet_pm), gated by CONNECTOR_LISTS.INCLUDE.CLIENT_AUTH_CONFIRM.
 */
export const connectorDetails = {
  wallet_pm: {
    Create: {
      Request: {
        amount: 100,
        currency: "GBP",
        payout_type: "wallet",
        payout_method_data: {
          wallet: {
            paypal: wallet_data,
          },
        },
        entity_type: "NaturalPerson",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_confirmation",
          payout_type: "wallet",
        },
      },
    },
    ConfirmClientAuth: {
      Request: {
        payout_type: "wallet",
        payout_method_data: {
          wallet: {
            paypal: wallet_data,
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
          payout_type: "wallet",
        },
      },
    },
    ConfirmClientAuthRestrictedFields: {
      Request: {
        amount: 10000,
        payout_type: "wallet",
        payout_method_data: {
          wallet: {
            paypal: wallet_data,
          },
        },
      },
      Response: {
        status: 422,
        body: {
          error: {
            type: "invalid_request",
            message:
              "The following fields cannot be provided for client-authenticated payout confirmation: amount",
            code: "IR_06",
          },
        },
      },
    },
    ConfirmClientAuthMultipleRestrictedFields: {
      Request: {
        amount: 10000,
        currency: "USD",
        auto_fulfill: false,
        payout_type: "wallet",
        payout_method_data: {
          wallet: {
            paypal: wallet_data,
          },
        },
      },
      Response: {
        status: 422,
        body: {
          error: {
            type: "invalid_request",
            message:
              "The following fields cannot be provided for client-authenticated payout confirmation: amount, auto_fulfill, currency",
            code: "IR_06",
          },
        },
      },
    },
    ConfirmClientAuthMissingClientSecret: {
      Request: {
        client_secret: null,
        amount: 10000,
        payout_type: "wallet",
        payout_method_data: {
          wallet: {
            paypal: wallet_data,
          },
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Missing required param: client_secret",
            code: "IR_04",
          },
        },
      },
    },
    ConfirmMerchantAuth: {
      Request: {
        amount: 100,
        payout_type: "wallet",
        payout_method_data: {
          wallet: {
            paypal: wallet_data,
          },
        },
      },
      Response: {
        status: 200,
        body: {
          status: "pending",
          payout_type: "wallet",
        },
      },
    },
    RetrieveAfterClientAuthErrors: {
      Response: {
        status: 200,
        body: {
          status: "requires_confirmation",
          payout_type: "wallet",
        },
      },
    },
    RetrieveAfterConfirm: {
      Response: {
        status: 200,
        body: {
          status: "pending",
          payout_type: "wallet",
        },
      },
    },
  },
};
