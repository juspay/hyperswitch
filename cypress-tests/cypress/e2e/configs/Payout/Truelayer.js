const card_data = {
  card_number: "4111111111111111",
  expiry_month: "3",
  expiry_year: "2030",
  card_holder_name: "John Smith",
};

const payment_card_data = {
  card_number: "4111111111111111",
  card_exp_month: "03",
  card_exp_year: "2030",
  card_holder_name: "John Doe",
};

const billing = {
  address: {
    line1: "123 Main St",
    line2: "Apt 4B",
    city: "Los Angeles",
    state: "GB",
    zip: "90001",
    country: "NL",
    first_name: "John",
    last_name: "Doe",
  },
};

const passthrough_data = {
  psp_token: "0f03fd53-c3c9-5a84-a406-1082bfd65c7a",
  psp_customer_id: "a29d2d16-15e2-4b66-8646-bc29e206210a",
  token_type: "open_banking_uk",
};

const error = {
  type: "invalid_request",
  message: "Payout Eligibility for Truelayer is not implemented",
  code: "IR_00",
};

export const connectorDetails = {
  card_pm: {
    Create: {
      Request: {
        payout_method_data: {
          card: card_data,
        },
        currency: "EUR",
        payout_type: "card",
      },
      Response: {
        status: 501,
        body: {
          error: error,
        },
      },
    },
    Confirm: {
      Request: {
        payout_method_data: {
          card: card_data,
        },
        currency: "EUR",
        payout_type: "card",
      },
      Response: {
        status: 501,
        body: {
          error: error,
        },
      },
    },
    Fulfill: {
      Request: {
        payout_method_data: {
          card: card_data,
        },
        currency: "EUR",
        payout_type: "card",
        recurring: true,
      },
      Response: {
        status: 501,
        body: {
          error: error,
        },
      },
    },
    SavePayoutMethod: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        card: payment_card_data,
        metadata: {
          city: "NY",
          unit: "245",
        },
      },
      Response: {
        status: 200,
      },
    },
    Token: {
      Request: {
        payout_token: "token",
        payout_type: "card",
      },
      Response: {
        status: 501,
        body: {
          error: error,
        },
      },
    },
  },
  bank_transfer_pm: {
    open_banking: {
      Create: {
        Request: {
          amount: 10,
          currency: "GBP",
          connector: ["truelayer"],
          payout_type: "bank_redirect",
          payout_method_data: {
            passthrough: passthrough_data,
          },
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_confirmation",
            payout_type: "bank_redirect",
          },
        },
      },
      Confirm: {
        Request: {
          amount: 10,
          currency: "GBP",
          connector: ["truelayer"],
          payout_type: "bank_redirect",
          payout_method_data: {
            passthrough: passthrough_data,
          },
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_fulfillment",
            payout_type: "bank_redirect",
          },
        },
      },
      Fulfill: {
        Request: {
          amount: 10,
          currency: "GBP",
          connector: ["truelayer"],
          payout_type: "bank_redirect",
          payout_method_data: {
            passthrough: passthrough_data,
          },
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "initiated",
            payout_type: "bank_redirect",
          },
        },
      },
      SavePayoutMethod: {
        Request: {
          payment_method: "bank_transfer",
          payment_method_type: "open_banking",
          bank_transfer: passthrough_data,
        },
        Response: {
          status: 200,
          body: {
            payment_method: "bank_transfer",
            payment_method_type: "open_banking",
          },
        },
      },
      Token: {
        Request: {
          payout_token: "token",
          payout_type: "bank",
        },
        Response: {
          status: 200,
          body: {
            status: "initiated",
            payout_type: "bank",
          },
        },
      },
      InvalidBillingDescriptorConfirm: {
        Request: {
          amount: 10,
          currency: "GBP",
          connector: ["truelayer"],
          payout_type: "bank_redirect",
          payout_method_data: {
            passthrough: passthrough_data,
          },
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_fulfillment",
            payout_type: "bank_redirect",
          },
        },
      },
      InvalidBillingDescriptor: {
        Request: {
          billing_descriptor: {
            reference: "invalid reference!!!",
          },
        },
        Response: {
          status: 422,
          body: {
            error: {
              code: "IR_16",
              message:
                "alphanumeric, hyphen, or period and length between 1 and 18 characters",
            },
          },
        },
      },
    },
    sepa_bank_transfer: {
      Create: {
        Configs: {
          TRIGGER_SKIP: true,
        },
        Request: {
          currency: "EUR",
          payout_type: "bank",
          payout_method_data: {
            bank_transfer: {
              payout_method_type: "sepa_bank_transfer",
              iban: "DE57331060435647542639",
            },
          },
          billing: billing,
        },
        Response: {
          status: 501,
          body: {
            error: error,
          },
        },
      },
      Confirm: {
        Configs: {
          TRIGGER_SKIP: true,
        },
        Request: {
          currency: "EUR",
          payout_type: "bank",
          payout_method_data: {
            bank_transfer: {
              payout_method_type: "sepa_bank_transfer",
              iban: "DE57331060435647542639",
            },
          },
          billing: billing,
        },
        Response: {
          status: 501,
          body: {
            error: error,
          },
        },
      },
      Fulfill: {
        Configs: {
          TRIGGER_SKIP: true,
        },
        Request: {
          currency: "EUR",
          payout_type: "bank",
          payout_method_data: {
            bank_transfer: {
              payout_method_type: "sepa_bank_transfer",
              iban: "DE57331060435647542639",
            },
          },
          billing: billing,
        },
        Response: {
          status: 501,
          body: {
            error: error,
          },
        },
      },
    },
  },
};
