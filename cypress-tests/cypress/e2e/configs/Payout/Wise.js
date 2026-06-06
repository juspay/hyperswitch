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
      // Payout Recurring test scenarios
      // Note: TRIGGER_SKIP is set because Wise sandbox returns failed for recurring
      // payouts with this test IBAN. Remove when cassettes are re-recorded with
      // a Wise sandbox account that supports recurring payout flows.
      RecurringTrue: {
        Configs: {
          TRIGGER_SKIP: true,
        },
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
        Configs: {
          TRIGGER_SKIP: true,
        },
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
        Configs: {
          TRIGGER_SKIP: true,
        },
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
      // Note: RecurringInvalidConfirm tests that confirm=false is rejected when
      // payout_method_id is provided. The payout_method_id is injected from
      // globalState (saved by the RecurringTrue test) so it passes deserialization
      // and the confirm=false validation runs and returns the expected error.
      RecurringInvalidConfirm: {
        Configs: {
          TRIGGER_SKIP: true,
        },
        Request: {
          currency: "EUR",
          payout_type: "bank",
          confirm: false,
        },
        Response: {
          status: 400,
          body: {
            error: {
              type: "invalid_request",
              message: "Confirm must be true for recurring payouts",
              code: "IR_06",
            },
          },
        },
      },
      // RecurringUseMethod tests the happy-path recurring payout flow: using a
      // payout_method_id saved from a prior RecurringTrue payout to create a new
      // payout without re-supplying full bank details.
      RecurringUseMethod: {
        Configs: {
          TRIGGER_SKIP: true,
        },
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
};
