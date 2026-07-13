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
      // RecurringTrue/False/Default test the recurring flag field behaviour.
      RecurringTrue: {
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
          confirm: true,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_fulfillment",
            payout_type: "bank",
            recurring: true,
          },
        },
      },
      RecurringFalse: {
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
          recurring: false,
          confirm: true,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_fulfillment",
            payout_type: "bank",
            recurring: false,
          },
        },
      },
      RecurringDefault: {
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
          confirm: true,
        },
        Response: {
          status: 200,
          body: {
            status: "requires_fulfillment",
            payout_type: "bank",
            recurring: false,
          },
        },
      },
      // RecurringInvalidConfirm is a negative test case — it expects a 422 error.
      // Do NOT add TRIGGER_SKIP or should_continue_further guards here.
      // Pattern matches EntityTypeInvalid in 00008-EntityType.cy.js.
      RecurringInvalidConfirm: {
        Request: {
          currency: "EUR",
          payout_type: "bank",
          confirm: false,
        },
        Response: {
          status: 422,
          body: {
            error: {
              type: "invalid_request",
              message: "Confirm must be true for recurring payouts",
              code: "IR_06",
            },
          },
        },
      },
      // The payout_method_id is saved by SavePayoutMethod and injected
      // from globalState at test runtime.
      RecurringUseMethod: {
        Request: {
          currency: "EUR",
          payout_type: "bank",
          // payout_method_id is injected from globalState at test runtime
        },
        Response: {
          status: 200,
          body: {
            status: "requires_fulfillment",
            payout_type: "bank",
            recurring: true,
          },
        },
      },
      EntityTypeCompany: {
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
          entity_type: "Company",
        },
        Response: {
          status: 200,
          body: {
            payout_type: "bank",
          },
        },
      },
      EntityTypeDefault: {
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
            payout_type: "bank",
          },
        },
      },
      EntityTypeIndividual: {
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
          entity_type: "Individual",
        },
        Response: {
          status: 200,
          body: {
            payout_type: "bank",
          },
        },
      },
      EntityTypeInvalid: {
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
          entity_type: "InvalidType",
        },
        Response: {
          status: 400,
          body: {
            error: {
              message: "Json deserialize error: unknown variant `InvalidType`",
              code: "IR_06",
            },
          },
        },
      },
      EntityTypeNaturalPerson: {
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
          entity_type: "NaturalPerson",
        },
        Response: {
          status: 200,
          body: {
            payout_type: "bank",
          },
        },
      },
      EntityTypeNonProfit: {
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
          entity_type: "NonProfit",
        },
        Response: {
          status: 200,
          body: {
            payout_type: "bank",
          },
        },
      },
      EntityTypePersonal: {
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
          entity_type: "Personal",
        },
        Response: {
          status: 200,
          body: {
            payout_type: "bank",
          },
        },
      },
      EntityTypePublicSector: {
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
          entity_type: "PublicSector",
        },
        Response: {
          status: 200,
          body: {
            payout_type: "bank",
          },
        },
      },
    },
  },
  payout_link_pm: {
    PayoutLinkBase: {
      Request: {
        payout_link: true,
        currency: "EUR",
        payout_type: "bank",
        payout_link_config: {
          test_mode: true,
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payout_method_data",
          payout_link: {
            payout_link_id: ".*",
            link: ".*",
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
          enabled_payment_methods: [
            {
              payment_method: "bank_transfer",
              payment_method_types: ["sepa_bank_transfer"],
            },
          ],
        },
      },
      Response: {
        status: 200,
        body: {
          status: "requires_payout_method_data",
          payout_link: {
            payout_link_id: ".*",
            link: ".*",
          },
        },
      },
      // BankData holds the test bank credentials (IBAN, BIC) required by the
      // handlePayoutLinkBankRedirection command to simulate a user filling
      // the SEPA bank transfer form on the hosted payout link page.
      // It is placed outside Request/Response because it is neither sent in
      // the API payload nor asserted in the API response.
      BankData: {
        iban: "NL46TEST0136169112",
        bic: "ABNANL2A",
        bank_name: "Test Bank",
        bank_country_code: "NL",
        bank_city: "Amsterdam",
      },
    },
    PayoutLinkValidationError: {
      Request: {
        payout_link: true,
        currency: "EUR",
        amount: 100,
        customer_id: null,
        payout_link_config: {
          test_mode: true,
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            code: "IR_04",
            message:
              "Missing required param: customer or customer_id when payout_link is true",
          },
        },
      },
    },
    PayoutLinkConfirmConflict: {
      Request: {
        payout_link: true,
        currency: "EUR",
        amount: 100,
        confirm: true,
        payout_link_config: {
          test_mode: true,
        },
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "cannot confirm a payout while creating a payout link",
            code: "IR_06",
          },
        },
      },
    },
    PayoutLinkWithoutLink: {
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
