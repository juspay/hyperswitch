const wallet_data = {
  telephone_number: "16608213349",
};

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
