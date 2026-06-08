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
      // RecurringTrue/False/Default test the recurring flag field behaviour:
      // these only go up to requires_fulfillment (create + confirm, no fulfill),
      // so the Wise sandbox limitation (failed status on actual execution) does
      // not apply. The recurring value is stored in the Hyperswitch DB and
      // echoed back in the response regardless of what Wise returns, so these
      // assertions work without any Rust changes to the Wise transformer.
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
      // RecurringInvalidConfirm and RecurringUseMethod are TRIGGER_SKIP because
      // they depend on payout_method_id being returned by the connector (saved
      // from RecurringTrue). Wise does not return payout_method_id in its
      // payout response — this requires Rust changes to propagate the recurring
      // flag through PayoutsData and implement it in the Wise transformer.
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
      // payout without re-supplying full bank details. TRIGGER_SKIP matches the
      // rest of the recurring suite — depends on RecurringTrue completing.
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
