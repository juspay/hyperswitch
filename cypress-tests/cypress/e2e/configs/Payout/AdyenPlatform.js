const card_data = {
  card_number: "4111111111111111",
  expiry_month: "03",
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
    line1: "Raadhuisplein",
    line2: "92",
    city: "Hoogeveen",
    state: "FL",
    zip: "7901 BW",
    country: "NL",
    first_name: "John",
    last_name: "Doe",
  },
  phone: {
    number: "9123456789",
    country_code: "+31",
  },
};

const PaymentMethodData = {
  card: {
    card_issuer: "CONOTOXIA SP Z O.O.",
    card_network: "Visa",
    card_type: "DEBIT",
    card_issuing_country: "POLAND",
    bank_code: "JP_JPM",
    last4: "1111",
    card_isin: "411111",
    card_extended_bin: null,
    card_exp_month: "03",
    card_exp_year: "2030",
    card_holder_name: "John Smith",
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
        currency: "EUR",
        payout_type: "card",
      },
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          payout_type: "card",
          payout_method_data: PaymentMethodData,
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
        status: 200,
        body: {
          status: "initiated",
          payout_type: "card",
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
        status: 200,
        body: {
          status: "initiated",
          payout_type: "card",
        },
      },
    },
  },
  bank_transfer_pm: {
    sepa_bank_transfer: {
      Create: {
        Request: {
          payout_type: "bank",
          priority: "instant",
          payout_method_data: {
            bank: {
              iban: "NL57INGB4654188101",
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
          priority: "instant",
          payout_method_data: {
            bank: {
              iban: "NL57INGB4654188101",
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
          priority: "instant",
          payout_method_data: {
            bank: {
              iban: "NL57INGB4654188101",
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
            iban: "NL57INGB4654188101",
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
          payout_token: "token",
          payout_type: "bank",
          priority: "instant",
        },
        Response: {
          status: 200,
          body: {
            status: "initiated",
            payout_type: "bank",
          },
        },
      },
      EntityTypeIndividual: {
        Request: {
          payout_type: "bank",
          priority: "regular",
          payout_method_data: {
            bank: {
              iban: "NL57INGB4654188101",
            },
          },
          billing: billing,
          entity_type: "Individual",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_confirmation",
            payout_type: "bank",
          },
        },
      },
      EntityTypeCompany: {
        Request: {
          payout_type: "bank",
          priority: "regular",
          payout_method_data: {
            bank: {
              iban: "NL57INGB4654188101",
            },
          },
          billing: billing,
          entity_type: "Company",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_confirmation",
            payout_type: "bank",
          },
        },
      },
      EntityTypeNonProfit: {
        Request: {
          payout_type: "bank",
          priority: "regular",
          payout_method_data: {
            bank: {
              iban: "NL57INGB4654188101",
            },
          },
          billing: billing,
          entity_type: "NonProfit",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_confirmation",
            payout_type: "bank",
          },
        },
      },
      EntityTypePublicSector: {
        Request: {
          payout_type: "bank",
          priority: "regular",
          payout_method_data: {
            bank: {
              iban: "NL57INGB4654188101",
            },
          },
          billing: billing,
          entity_type: "PublicSector",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_confirmation",
            payout_type: "bank",
          },
        },
      },
      EntityTypeNaturalPerson: {
        Request: {
          payout_type: "bank",
          priority: "regular",
          payout_method_data: {
            bank: {
              iban: "NL57INGB4654188101",
            },
          },
          billing: billing,
          entity_type: "NaturalPerson",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_confirmation",
            payout_type: "bank",
          },
        },
      },
      EntityTypePersonal: {
        Request: {
          payout_type: "bank",
          priority: "regular",
          payout_method_data: {
            bank: {
              iban: "NL57INGB4654188101",
            },
          },
          billing: billing,
          entity_type: "Personal",
        },
        Response: {
          status: 200,
          body: {
            status: "requires_confirmation",
            payout_type: "bank",
          },
        },
      },
      EntityTypeDefault: {
        Request: {
          payout_type: "bank",
          priority: "regular",
          payout_method_data: {
            bank: {
              iban: "NL57INGB4654188101",
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
      EntityTypeInvalid: {
        Request: {
          payout_type: "bank",
          priority: "regular",
          payout_method_data: {
            bank: {
              iban: "NL57INGB4654188101",
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
    },
    PayoutPriority: {
      Configs: {
        // AdyenPlatform test IBAN does not support instant priority payouts
        TRIGGER_SKIP: true,
      },
      Request: {
        payout_type: "bank",
        priority: "instant",
        payout_method_data: {
          bank: {
            iban: "NL57INGB4654188101",
          },
        },
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          status: "initiated",
          priority: "instant",
          payout_type: "bank",
        },
      },
    },
    PayoutPriorityMissing: {
      Request: {
        payout_type: "bank",
        payout_method_data: {
          bank: {
            iban: "NL57INGB4654188101",
          },
        },
        billing: billing,
      },
      Response: {
        status: 400,
        body: {
          error: {
            type: "invalid_request",
            message: "Missing required param: priority",
            code: "IR_04",
          },
        },
      },
    },
    PayoutPriorityRegular: {
      Request: {
        payout_type: "bank",
        priority: "regular",
        payout_method_data: {
          bank: {
            iban: "NL57INGB4654188101",
          },
        },
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          priority: "regular",
          payout_type: "bank",
        },
      },
    },
    PayoutPriorityWire: {
      Request: {
        payout_type: "bank",
        priority: "wire",
        payout_method_data: {
          bank: {
            iban: "NL57INGB4654188101",
          },
        },
        billing: billing,
      },
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          priority: "wire",
          payout_type: "bank",
        },
      },
    },
    RetrievePriorityInstant: {
      Response: {
        status: 200,
        body: {
          status: "initiated",
          priority: "instant",
        },
      },
    },
    RetrievePriorityRegular: {
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          priority: "regular",
        },
      },
    },
    RetrievePriorityWire: {
      Response: {
        status: 200,
        body: {
          status: "requires_fulfillment",
          priority: "wire",
        },
      },
    },
  },
};
