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
 * (wallet_pm), gated by CONNECTOR_LISTS.INCLUDE.CLIENT_AUTH_CONFIRM, and in
 * the full wallet payout lifecycle spec 00010-WalletPayout.cy.js (create ->
 * confirm -> fulfill -> sync -> retrieve, PayPal and Venmo wallet variants),
 * gated by CONNECTOR_LISTS.INCLUDE.WALLET_PAYOUT.
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
    // Full wallet payout lifecycle (merchant-auth, manual fulfill):
    // create -> confirm -> fulfill -> force-sync retrieve. Consumed by the
    // wallet payout spec (00010-WalletPayout.cy.js), gated by
    // CONNECTOR_LISTS.INCLUDE.WALLET_PAYOUT. Response bodies for
    // Fulfill / RetrieveAfterFulfill / SyncAfterFulfill reflect the PayPal
    // sandbox behaviour verified in the CLA-440 pipeline runs (payouts stay
    // "pending" until the sandbox receiver claims them) and must be
    // re-verified against the live server if the sandbox changes.
    Confirm: {
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
          status: "requires_fulfillment",
          payout_type: "wallet",
        },
      },
    },
    Fulfill: {
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
    RetrieveAfterFulfill: {
      Response: {
        status: 200,
        body: {
          status: "pending",
          payout_type: "wallet",
        },
      },
    },
    SyncAfterFulfill: {
      Response: {
        status: 200,
        body: {
          status: "pending",
          payout_type: "wallet",
        },
      },
    },
    // Venmo wallet variants: Venmo payouts route through the same
    // PayoutMethodData::Wallet arm with recipient_wallet = Venmo and are
    // resolved from telephone_number only (no email / paypal_id). They use
    // the same paypal connector credentials — no separate creds.json key.
    // Sandbox support for Venmo payout items must be live-verified.
    CreateVenmo: {
      Request: {
        amount: 100,
        currency: "GBP",
        payout_type: "wallet",
        payout_method_data: {
          wallet: {
            venmo: wallet_data,
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
    ConfirmVenmo: {
      Request: {
        amount: 100,
        currency: "GBP",
        payout_type: "wallet",
        payout_method_data: {
          wallet: {
            venmo: wallet_data,
          },
        },
        entity_type: "NaturalPerson",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          payout_type: "wallet",
        },
      },
    },
    FulfillVenmo: {
      Request: {
        payout_type: "wallet",
        payout_method_data: {
          wallet: {
            venmo: wallet_data,
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
    // Negative: every paypal wallet receiver field is schema-optional, so a
    // payout created with an empty paypal receiver object is accepted at
    // create/confirm and reaches requires_fulfillment — but the fulfill
    // transformer (PaypalPayoutItem::try_from) requires at least one receiver
    // identifier (email / telephone_number / paypal_id) and fails with
    // MissingRequiredField "receiver_data", which the router surfaces as
    // 400 IR_04. The payout silently stays in requires_fulfillment.
    MissingReceiverDataConfirm: {
      Request: {
        amount: 100,
        currency: "GBP",
        payout_type: "wallet",
        payout_method_data: {
          wallet: {
            paypal: {},
          },
        },
        entity_type: "NaturalPerson",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          payout_type: "wallet",
        },
      },
    },
    MissingReceiverDataFulfill: {
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_04",
            message: "Missing required param: receiver_data",
          },
        },
      },
    },
    MissingReceiverDataRetrieveAfterFulfill: {
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          payout_type: "wallet",
        },
      },
    },
  },
};
