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
    line1: "1467",
    line2: "Harrison Street",
    line3: "Harrison Street",
    city: "San Fransico",
    state: "CA",
    zip: "94122",
    country: "US",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "9123456789",
    country_code: "+91",
  },
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
          error: {
            type: "invalid_request",
            message: `Payout Eligibility for Wise is not implemented`,
            code: "IR_00",
          },
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
          error: {
            type: "invalid_request",
            message: `Payout Eligibility for Wise is not implemented`,
            code: "IR_00",
          },
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
          error: {
            type: "invalid_request",
            message: `Payout Eligibility for Wise is not implemented`,
            code: "IR_00",
          },
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
          error: {
            type: "invalid_request",
            message: `Payout Eligibility for Wise is not implemented`,
            code: "IR_00",
          },
        },
      },
    },
  },
  bank_transfer_pm: {
    sepa_bank_transfer: {
      Create: {
        Request: {
          currency: "EUR",
          payout_type: "bank",
          payout_method_data: {
            bank: {
              iban: "NL46TEST0136169112",
              bic: "ABNANL2A",
              bank_name: "Test Bank",
              bank_country_code: "NL",
              bank_city: "Amsterdam",
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
          currency: "EUR",
          payout_type: "bank",
          payout_method_data: {
            bank: {
              iban: "NL46TEST0136169112",
              bic: "ABNANL2A",
              bank_name: "Test Bank",
              bank_country_code: "NL",
              bank_city: "Amsterdam",
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
          currency: "EUR",
          payout_type: "bank",
          payout_method_data: {
            bank: {
              iban: "NL46TEST0136169112",
              bic: "ABNANL2A",
              bank_name: "Test Bank",
              bank_country_code: "NL",
              bank_city: "Amsterdam",
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
      },
      SavePayoutMethod: {
        Request: {
          payment_method: "bank_transfer",
          payment_method_type: "sepa_bank_transfer",
          bank_transfer: {
            iban: "NL46TEST0136169112",
            bic: "ABNANL2A",
            bank_name: "Deutsche Bank",
            bank_country_code: "NL",
            bank_city: "Amsterdam",
          },
        },
        Response: {
          status: 200,
          body: {
            payment_method: "bank_transfer",
            payment_method_type: "sepa_bank_transfer",
          },
        },
      },
      Token: {
        Request: {
          currency: "EUR",
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
  payout_link_pm: {
    PayoutLinkBase: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payout_link: true,
        currency: "EUR",
        payout_type: "bank",
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_00",
            message: "Payout Eligibility for Wise is not implemented",
          },
        },
      },
    },
    PayoutLinkBankTransfer: {
      Request: {
        payout_link: true,
        currency: "EUR",
        amount: 100,
        description: "Test Payout Link Bank Transfer",
        payout_link_config: {
          test_mode: true,
          enabled_payment_methods: ["bank_transfer"],
        },
      },
      Response: {
        status: 200,
      },
      BankData: {
        iban: "NL46TEST0136169112",
        bic: "ABNANL2A",
        bank_name: "Test Bank",
        bank_country_code: "NL",
        bank_city: "Amsterdam",
      },
    },
    PayoutLinkValidationError: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payout_link: true,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_04",
            message:
              "Provide either customer or customer_id when payout_link is true",
          },
        },
      },
    },
    PayoutLinkConfirmConflict: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payout_link: true,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_04",
            message: "Cannot confirm a payout while creating a payout link",
          },
        },
      },
    },
    PayoutLinkWithoutLink: {
      Configs: {
        TRIGGER_SKIP: true,
      },
      Request: {
        payout_link: false,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payout_method_data",
        },
      },
    },
  },
};
