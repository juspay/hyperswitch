// Card data for payout_method_data.card — uses payout-specific field names
// (card_number, expiry_month, expiry_year, card_holder_name)
const card_data = {
  card_number: "4000000000000077",
  expiry_month: "3",
  expiry_year: "2030",
  card_holder_name: "John Smith",
};

// Card data for SavePayoutMethod — uses payment-method field names
// (card_number, card_exp_month, card_exp_year, card_holder_name)
const payment_card_data = {
  card_number: "4000000000000077",
  card_exp_month: "03",
  card_exp_year: "2030",
  card_holder_name: "John Doe",
};

const billing = {
  address: {
    line1: "123 Main St",
    line2: "Apt 4B",
    city: "San Francisco",
    state: "CA",
    zip: "94102",
    country: "US",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "4155551234",
    country_code: "+1",
  },
};

export const connectorDetails = {
  card_pm: {
    Create: {
      Request: {
        payout_method_data: {
          card: card_data,
        },
        currency: "USD",
        payout_type: "card",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_confirmation",
          payout_type: "card",
        },
      },
    },
    Confirm: {
      Request: {
        payout_method_data: {
          card: card_data,
        },
        currency: "USD",
        payout_type: "card",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          payout_type: "card",
        },
      },
    },
    Fulfill: {
      Request: {
        payout_method_data: {
          card: card_data,
        },
        currency: "USD",
        payout_type: "card",
        recurring: true,
      },
      Response: {
        status: 200,
        body: {
          status: "initiated",
          payout_type: "card",
        },
      },
      Configs: {
        TRIGGER_SKIP: true,
      },
    },
    SavePayoutMethod: {
      Request: {
        payment_method: "card",
        payment_method_type: "credit",
        card: payment_card_data,
        metadata: {
          city: "SF",
          unit: "4B",
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
        status: 200,
        body: {
          status: "initiated",
          payout_type: "card",
        },
      },
    },
  },
  bank_transfer_pm: {
    ach_bank_transfer: {
      Create: {
        Request: {
          payout_type: "bank",
          priority: "regular",
          payout_method_data: {
            bank: {
              account_number: "000123456789",
              routing_number: "110000000",
            },
          },
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_confirmation",
            payout_type: "bank",
          },
        },
      },
      Confirm: {
        Request: {
          payout_type: "bank",
          priority: "regular",
          payout_method_data: {
            bank: {
              account_number: "000123456789",
              routing_number: "110000000",
            },
          },
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_fulfillment",
            payout_type: "bank",
          },
        },
      },
      Fulfill: {
        Request: {
          payout_type: "bank",
          priority: "regular",
          payout_method_data: {
            bank: {
              account_number: "000123456789",
              routing_number: "110000000",
            },
          },
          billing: billing,
          recurring: true,
        },
        Response: {
          status: 200,
          body: {
            status: "initiated",
            payout_type: "bank",
          },
        },
        Configs: {
          TRIGGER_SKIP: true,
        },
      },
      SavePayoutMethod: {
        Request: {
          payment_method: "bank_transfer",
          payment_method_type: "ach_bank_transfer",
          bank_transfer: {
            account_number: "000123456789",
            routing_number: "110000000",
          },
        },
        Response: {
          status: 200,
          body: {
            payment_method: "bank_transfer",
            payment_method_type: "ach_bank_transfer",
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
            status: "success",
            payout_type: "bank",
          },
        },
      },
    },
    sepa_bank_transfer: {
      Create: {
        Request: {
          payout_type: "bank",
          priority: "regular",
          payout_method_data: {
            bank: {
              iban: "DE89370400440532013000",
            },
          },
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_confirmation",
            payout_type: "bank",
          },
        },
      },
      Confirm: {
        Request: {
          payout_type: "bank",
          priority: "regular",
          payout_method_data: {
            bank: {
              iban: "DE89370400440532013000",
            },
          },
          billing: billing,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_fulfillment",
            payout_type: "bank",
          },
        },
      },
      Fulfill: {
        Request: {
          payout_type: "bank",
          priority: "regular",
          payout_method_data: {
            bank: {
              iban: "DE89370400440532013000",
            },
          },
          billing: billing,
          recurring: true,
        },
        Response: {
          status: 200,
          body: {
            status: "initiated",
            payout_type: "bank",
          },
        },
        Configs: {
          TRIGGER_SKIP: true,
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
    },
  },
};
